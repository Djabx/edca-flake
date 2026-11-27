// The post effects' deband (edcas_post.hlsl), first in the chain: haasn's deband (Niklas Haas, MIT; ReShade's
// Deband.fx 3.1 by JPulowski, credit to the original authors) with its defaults: four reads at a random distance up
// to 24 px in a random direction, their mean taken where the area is flat (Weber ratio under 0.04, deviation under
// 0.007; both times w602), plus CeeJay.dk's ordered dither of a quarter of an 8-bit step. Against the 8-bit
// banding of Elite's skies and nebulae. Its random number is the frame's time instead of ReShade's per-frame random.
#include "edcas_post.hlsl"

float Rand(float x)
{
	return frac(x / 41.0);
}

float Permute(float x)
{
	return ((34.0 * x + 1.0) * x) % 289.0;
}

float4 main(float4 pos : SV_Position) : SV_Target0
{
	float4 size = PictureSize();
	float2 uv = pos.xy * size.zw;
	float4 picture = Picture.Load(int3(pos.xy, 0));
	float3 ori = picture.rgb;
	float strength = PostRow(602).w;
	const float range = 24.0;
	float drandom = frac(PostRow(601).w * 0.6180339887) * 32767.0;
	float3 m = float3(uv + 1.0, drandom / 32767.0 + 1.0);
	float h = Permute(Permute(Permute(m.x) + m.y) + m.z);
	float dir = Rand(Permute(h)) * 6.2831853;
	float2 o;
	sincos(dir, o.y, o.x);
	float2 pt = Rand(h) * range * size.zw;
	float3 ref0 = PictureAt(uv + pt * o), ref1 = PictureAt(uv - pt * o);
	float3 ref2 = PictureAt(uv + pt * float2(-o.y, o.x)), ref3 = PictureAt(uv + pt * float2(o.y, -o.x));
	float3 mean = (ori + ref0 + ref1 + ref2 + ref3) * 0.2;
	float3 k = abs(ori - mean) + abs(ref0 - mean) + abs(ref1 - mean) + abs(ref2 - mean) + abs(ref3 - mean);
	k = k * 0.2 / max(mean, 1e-5);
	float3 sd = sqrt((pow(ref0 - ori, 2) + pow(ref1 - ori, 2) + pow(ref2 - ori, 2) + pow(ref3 - ori, 2)) * 0.25);
	float3 smooth = (ref0 + ref1 + ref2 + ref3) * 0.25;
	bool3 flat = k <= 0.04 * strength && sd <= 0.007 * strength;
	float gp = frac(dot(uv, size.xy * float2(1.0 / 16.0, 10.0 / 36.0) + 0.25));
	float ds = 0.25 / 255.0;
	float3 dither = lerp(2.0 * float3(ds, -ds, ds), -2.0 * float3(ds, -ds, ds), gp);
	return float4(flat ? saturate(smooth + dither) : ori, picture.a);
}
