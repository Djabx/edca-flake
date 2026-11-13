// Lays the clouds and the fog over the picture (edcas.ini [CustomShaderEdcasCloudsComposite]): the
// half-size buffer edcas_clouds_final.hlsl drew this frame, upscaled over the 4 x 4 texels around
// each pixel with a tent 1.75 texels wide, which also smooths the march's grain (2026-10-26). The
// march at full size over the whole picture took Elite from over 200 FPS to 86; Elite draws its
// own sky smaller still.
// The texels count by how near their depth is to the pixel's: plain upscaling mixed the fog
// behind a ship into the texels on its edge and drew a dark, blocky fringe round it (2026-10-27).
// Runs on the geometry of Elite's atmosphere pass (draw = from_caller), of its sky upsample, or on
// a full-screen triangle (edcas.ini, 2026-11-08): it reads nothing of a vertex shader but the pixel
// position, and its constants and depth buffers are bound by name. Within a band only when x525 is
// set: the pixels ResourceEdcasBandMask marks (edcas_copy.hlsl).

cbuffer cb1 : register(b1)
{
	float4 cb1[334];
}

Texture2D<float4> Depth : register(t0);
Texture2D<float4> DepthAlt : register(t4);
Texture2D<float4> Clouds : register(t110);
Texture2D<float> Mask : register(t111);
Texture1D<float4> IniParams : register(t120);

// The depth at a point of the picture, as edcas_clouds.hlsl reads it.
float depthAt(float2 uv, bool alt, float2 size)
{
	int3 pixel = int3(min(uv * size, size - 1), 0);
	// Held below infinity, should the sky be stored as that: inf - inf would poison the weights.
	return min(alt ? DepthAlt.Load(pixel).x : Depth.Load(pixel).x, 1e30);
}

void main(float4 pos : SV_Position, out float4 o0 : SV_Target0)
{
	if (IniParams.Load(int2(525, 0)).x > 0 && Mask.Load(int3(pos.xy, 0)) <= 0)
		discard;
	uint width, height;
	Clouds.GetDimensions(width, height);
	float2 halfSize = float2(width, height);
	int2 last = int2(width - 1, height - 1);
	bool alt = asint(cb1[25].w) == 1;
	uint depthWidth, depthHeight;
	if (alt)
		DepthAlt.GetDimensions(depthWidth, depthHeight);
	else
		Depth.GetDimensions(depthWidth, depthHeight);
	float2 size = float2(depthWidth, depthHeight);

	// The pixel's place in the picture, which the depth buffer covers at the picture's size.
	float2 uv = pos.xy / size;
	float depth = depthAt(uv, alt, size);
	float2 at = uv * halfSize - 0.5;
	int2 base = int2(floor(at));

	float4 sum = 0, nearest = 0;
	float weights = 0, closest = 1e30;
	[unroll] for (int y = -1; y <= 2; y++)
		[unroll] for (int x = -1; x <= 2; x++) {
			int2 t = clamp(base + int2(x, y), 0, last);
			float2 d = abs(float2(t) - at);
			float tent = max(1.75 - d.x, 0) * max(1.75 - d.y, 0);
			float texelDepth = depthAt((float2(t) + 0.5) / halfSize, alt, size);
			float apart = abs(texelDepth - depth) / max(min(texelDepth, depth), 1e-6);
			float4 c = Clouds.Load(int3(t, 0));
			float w = tent * exp(-10 * apart);
			sum += w * c;
			weights += w;
			if (apart < closest) {
				closest = apart;
				nearest = c;
			}
		}
	// No texel near the pixel's depth (a thin edge only this pixel sees): the nearest in depth.
	o0 = weights > 1e-4 ? sum / weights : nearest;
	// Light scattered by the medium comes without alpha (edcas_lights.hlsl).
	// Written as "not above 0" so that a NaN is dropped too.
	if (!(o0.a > 0) && !any(o0.rgb > 0))
		discard;
}
