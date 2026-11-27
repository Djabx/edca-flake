// What the post effects share (README.md "Post effects", 2026-11-25): the picture they read, the rows of
// their strengths, the full-screen pass's pixel. The effects run on Elite's tonemapped picture (the target of
// its tonemap draws 380822531cf1f3d4, dfa17e75becfdfbf, 5bcfb4f2136acea7: RGBA8, display-encoded values),
// before its 2D overlays, as ReShade's run on a game's back buffer: the formulas take the values as they are,
// gamma-encoded, as ReShade's do.
//
// The SweetFX effects follow CeeJay.dk's SweetFX shaders (https://github.com/CeeJayDK/SweetFX, MIT, Copyright (c)
// 2014 CeeJayDK; DPX by Loadus, Technicolor by DKT70, LiftGammaGain by 3an and CeeJay.dk, CAS a port of AMD's
// FidelityFX CAS, MIT), at their defaults; Deband follows haasn's (Niklas Haas, MIT, ReShade port by JPulowski),
// Daltonize daltonize.org's LMS algorithm, SMAA the reference implementation (edcas_smaa.hlsl); the Gaussian blur
// is EDCAS's own kernel.
//
// Rows (strength: 0 off, 1 the effect's usual strength, up to 3; client post_effects.rs):
//   x600 chromatic aberration   y600 DPX            z600 curves          w600 fake HDR
//   x601 vibrance               y601 film grain     z601 Gaussian blur   w601 `time` (the grain's seed)
//   x602 SMAA (on above 0)      y602 CAS            z602 LumaSharpen     w602 deband
//   x603 vignette               y603 levels         z603 lift/gamma/gain w603 Technicolor
//   x604 y604 z604 the lift     x605 y605 z605 the gamma                 x606 y606 z606 the gain (r, g, b; 1 neutral)
//   x607 Daltonize              y607 its type (0 protanopia, 1 deuteranopia, 2 tritanopia; set by the ini from
//                               $daltonize_type, which does not fade)
//   z607 1 when Elite's bloom is bound for the lens flare (the ini: $post_bloom)
//   x608 the supersampling downscale (edcas_downscale.hlsl)
//   x609 lens flare             y609 anamorphic streaks   z609 white balance (on)   w609 its temperature (K)
//   x610 bleach bypass          y610 monochrome           z610 sepia

Texture2D<float4> Picture : register(t110);
Texture1D<float4> IniParams : register(t120);

float4 PostRow(int row)
{
	return IniParams.Load(int2(row, 0));
}

// the picture's size in pixels and one over it
float4 PictureSize()
{
	float w, h;
	Picture.GetDimensions(w, h);
	return float4(w, h, 1 / w, 1 / h);
}

// The picture at uv, filtered bilinearly by hand: a custom pass has no sampler of its own (the one bound
// would be whatever Elite's tonemap left), and edges are clamped.
float3 PictureAt(float2 uv)
{
	float4 size = PictureSize();
	float2 p = uv * size.xy - 0.5;
	float2 f = frac(p);
	int2 a = int2(floor(p)), hi = int2(size.xy) - 1;
	int2 a0 = clamp(a, 0, hi), a1 = clamp(a + 1, 0, hi);
	float3 c00 = Picture.Load(int3(a0.x, a0.y, 0)).rgb, c10 = Picture.Load(int3(a1.x, a0.y, 0)).rgb;
	float3 c01 = Picture.Load(int3(a0.x, a1.y, 0)).rgb, c11 = Picture.Load(int3(a1.x, a1.y, 0)).rgb;
	return lerp(lerp(c00, c10, f.x), lerp(c01, c11, f.x), f.y);
}

// Any texture at uv, bilinear by hand (SMAA's lookups, edcas_smaa.hlsl), edges clamped.
float4 BilinearAt(Texture2D<float4> t, float2 uv)
{
	float w, h;
	t.GetDimensions(w, h);
	float2 p = uv * float2(w, h) - 0.5;
	float2 f = frac(p);
	int2 a = int2(floor(p)), hi = int2(w, h) - 1;
	int2 a0 = clamp(a, 0, hi), a1 = clamp(a + 1, 0, hi);
	float4 c00 = t.Load(int3(a0.x, a0.y, 0)), c10 = t.Load(int3(a1.x, a0.y, 0));
	float4 c01 = t.Load(int3(a0.x, a1.y, 0)), c11 = t.Load(int3(a1.x, a1.y, 0));
	return lerp(lerp(c00, c10, f.x), lerp(c01, c11, f.x), f.y);
}

// Any texture at uv, the nearest texel.
float4 PointAt(Texture2D<float4> t, float2 uv)
{
	float w, h;
	t.GetDimensions(w, h);
	int2 p = clamp(int2(floor(uv * float2(w, h))), 0, int2(w, h) - 1);
	return t.Load(int3(p, 0));
}

// sRGB's curve, both ways (CAS works in linear light)
float3 ToLinear(float3 c)
{
	c = saturate(c);
	return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4);
}

float3 ToSrgb(float3 c)
{
	c = saturate(c);
	return c <= 0.0031308 ? c * 12.92 : 1.055 * pow(c, 1 / 2.4) - 0.055;
}

// The picture at a pixel, clamped to the picture.
float3 PictureLoad(int2 p)
{
	int2 hi = int2(PictureSize().xy) - 1;
	return Picture.Load(int3(clamp(p, 0, hi), 0)).rgb;
}
