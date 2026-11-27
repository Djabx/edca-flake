// The supersampling downscale (README.md "Post effects", 2026-11-25): in place of Elite's copy of the finished
// picture to the back buffer, 2192bc2cb917f8bb (README-ELITE.md "Tonemap"), which takes ONE bilinear sample per
// output pixel. With supersampling above 1 that keeps 4 of the (1.5 x 1.5 =) 2.25 rendered pixels of an output
// pixel's footprint, at weights that change with the pixel's place: the extra samples are thrown away unevenly
// and edges still crawl. This filters the whole footprint with a Mitchell-Netravali kernel (B = C = 1/3) scaled
// to the ratio. At the same size it is Elite's copy pixel for pixel; smaller than the target (supersampling
// below 1) it is Elite's bilinear read. Drawn with Elite's own vertex shader, constants and t0 (draw =
// from_caller); x608 switches it on (edcas.ini). The ratio comes from the pixel's own step in the source's
// coordinates (ddx, ddy of uv), not from the target's size: a copy into part of a target (one eye of a VR pair,
// a letterbox) gets its own scale (code review 2026-11-26).
Texture2D<float4> Source : register(t0);

float Mitchell(float x)
{
	const float B = 1.0 / 3.0, C = 1.0 / 3.0;
	x = abs(x);
	if (x < 1)
		return ((12 - 9 * B - 6 * C) * x * x * x + (-18 + 12 * B + 6 * C) * x * x + (6 - 2 * B)) / 6;
	if (x < 2)
		return ((-B - 6 * C) * x * x * x + (6 * B + 30 * C) * x * x + (-12 * B - 48 * C) * x + (8 * B + 24 * C)) / 6;
	return 0;
}

float4 Bilinear(float2 uv, float2 size)
{
	float2 p = uv * size - 0.5;
	float2 f = frac(p);
	int2 a = int2(floor(p)), hi = int2(size) - 1;
	int2 a0 = clamp(a, 0, hi), a1 = clamp(a + 1, 0, hi);
	float4 c00 = Source.Load(int3(a0.x, a0.y, 0)), c10 = Source.Load(int3(a1.x, a0.y, 0));
	float4 c01 = Source.Load(int3(a0.x, a1.y, 0)), c11 = Source.Load(int3(a1.x, a1.y, 0));
	return lerp(lerp(c00, c10, f.x), lerp(c01, c11, f.x), f.y);
}

float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
	float w, h;
	Source.GetDimensions(w, h);
	float2 src = float2(w, h);
	// source pixels per target pixel along each axis (uv is linear over the triangle, so the quad's step is exact)
	float2 ratio = src * float2(abs(ddx(uv.x)), abs(ddy(uv.y)));
	if (ratio.x <= 1.01 && ratio.y <= 1.01)
		return Bilinear(uv, src);	// the same size or upscaling: Elite's own read
	// the kernel's support is 2 output pixels each side, 2 * ratio source pixels; at most 2.5x (11 taps a side)
	float2 r = min(ratio, 2.5);
	float2 centre = uv * src - 0.5;
	int2 first = int2(ceil(centre - 2 * r)), last = int2(floor(centre + 2 * r));
	int2 hi = int2(src) - 1;
	float4 sum = 0;
	float total = 0;
	[loop] for (int y = first.y; y <= last.y; y++)
	{
		float wy = Mitchell((y - centre.y) / r.y);
		[loop] for (int x = first.x; x <= last.x; x++)
		{
			float wgt = Mitchell((x - centre.x) / r.x) * wy;
			sum += wgt * Source.Load(int3(clamp(int2(x, y), 0, hi), 0));
			total += wgt;
		}
	}
	return saturate(sum / max(total, 1e-4));
}
