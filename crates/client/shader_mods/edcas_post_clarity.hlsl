// The post effects' clarity (README.md "Post effects", 2026-11-26): local contrast at a large scale, the picture's
// brightness against its surroundings some 10 to 40 pixels round, raised in the mid-tones (as Lightroom's Clarity
// and SweetFX's Clarity.fx do; EDCAS's own code). Three small passes at a quarter of the picture's size make the
// surroundings (edcas.ini ResourceEdcasPostClarity, ResourceEdcasPostClarityB): CLARITY_DOWN the brightness, each
// texel the mean of the 4 x 4 pixels it covers; CLARITY_H and CLARITY_V a Gaussian of sigma 3 texels (12 pixels)
// across and down. edcas_post_colour.hlsl reads the result (t115) and applies it, first of its effects.
#include "edcas_post.hlsl"

Texture2D<float4> Quarter : register(t115);

float Luma(float3 c)
{
	return dot(c, float3(0.2126, 0.7152, 0.0722));
}

float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
#if defined(CLARITY_DOWN)
	// 4 bilinear reads, each the mean of 2 x 2 pixels: the 4 x 4 the texel covers
	float2 px = PictureSize().zw;
	float l = 0;
	[unroll] for (int i = 0; i < 4; i++)
		l += Luma(PictureAt(uv + float2(i & 1 ? 1 : -1, i & 2 ? 1 : -1) * px).rgb);
	return float4(l / 4, 0, 0, 1);
#else
	float w, h;
	Quarter.GetDimensions(w, h);
#if defined(CLARITY_H)
	float2 step = float2(1 / w, 0);
#else
	float2 step = float2(0, 1 / h);
#endif
	// a Gaussian of sigma 3 texels, 13 taps, at texel centres (the step is whole texels)
	float sum = 0, total = 0;
	[unroll] for (int i = -6; i <= 6; i++)
	{
		float g = exp(-i * i / 18.0);
		sum += g * BilinearAt(Quarter, uv + step * i).r;
		total += g;
	}
	return float4(sum / total, 0, 0, 1);
#endif
}
