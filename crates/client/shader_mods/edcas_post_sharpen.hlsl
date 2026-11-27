// The post effects' sharpening (edcas_post.hlsl), after SMAA: AMD's Contrast Adaptive Sharpening (FidelityFX CAS,
// MIT, SweetFX's port CAS.fx: contrast 0, sharpening 1 times y602, in linear light as CAS.fx reads it), then
// SweetFX's LumaSharpen (pattern Normal, strength 0.65 times z602, clamp 0.035).
#include "edcas_post.hlsl"

float3 Cas(int2 p, float sharpening)
{
	float3 a = ToLinear(PictureLoad(p + int2(-1, -1))), b = ToLinear(PictureLoad(p + int2(0, -1))), c = ToLinear(PictureLoad(p + int2(1, -1)));
	float3 d = ToLinear(PictureLoad(p + int2(-1, 0))), e = ToLinear(PictureLoad(p)), f = ToLinear(PictureLoad(p + int2(1, 0)));
	float3 g = ToLinear(PictureLoad(p + int2(-1, 1))), h = ToLinear(PictureLoad(p + int2(0, 1))), i = ToLinear(PictureLoad(p + int2(1, 1)));
	float3 mn = min(min(min(d, e), min(f, b)), h);
	float3 mn2 = min(mn, min(min(a, c), min(g, i)));
	mn += mn2;
	float3 mx = max(max(max(d, e), max(f, b)), h);
	float3 mx2 = max(mx, max(max(a, c), max(g, i)));
	mx += mx2;
	float3 amp = saturate(min(mn, 2.0 - mx) / max(mx, 1e-5));
	amp = rsqrt(max(amp, 1e-8));
	const float peak = 8.0;	// -3 contrast + 8, contrast 0
	float3 w = -1 / (amp * peak);
	float3 window = (b + d) + (f + h);
	float3 o = saturate((window * w + e) / (4.0 * w + 1.0));
	return ToSrgb(lerp(e, o, sharpening));
}

// the sharpening of the picture as it came (not of CAS's result), added to c
float3 LumaSharpen(float3 c, float3 ori, float2 uv, float2 px, float strength)
{
	const float sharpClamp = 0.035;
	float3 ssl = float3(0.2126, 0.7152, 0.0722) * 0.65 * strength;
	float3 blur = PictureAt(uv + float2(px.x, -px.y) * 0.5) + PictureAt(uv - px * 0.5)
		+ PictureAt(uv + px * 0.5) + PictureAt(uv - float2(px.x, -px.y) * 0.5);
	blur *= 0.25;
	float3 sharp = ori - blur;
	float sharpLuma = clamp(dot(sharp, ssl), -sharpClamp, sharpClamp);
	return saturate(c + sharpLuma);
}

float4 main(float4 pos : SV_Position) : SV_Target0
{
	float4 size = PictureSize();
	float2 uv = pos.xy * size.zw;
	float4 picture = Picture.Load(int3(pos.xy, 0));
	float3 c = picture.rgb;
	float4 r602 = PostRow(602);
	if (r602.y > 0)
		c = saturate(Cas(int2(pos.xy), r602.y));
	if (r602.z > 0)
		c = LumaSharpen(c, picture.rgb, uv, size.zw, r602.z);
	return float4(c, picture.a);
}
