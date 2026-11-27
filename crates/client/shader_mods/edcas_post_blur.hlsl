// The post effects' Gaussian blur (edcas_post.hlsl): one direction of a separable Gaussian of sigma 3 px
// (about ReShade GaussianBlur's radius 1), 25 taps out to 4 sigma. Its strength's
// row blends it over the picture (0.5 at 1, all of it at 2, the row's top; ReShade's default of 0.3 was all but
// invisible in the game, 2026-11-26), in the vertical pass. EDCAS's own kernel:
// Ioxa's shader states no licence.
#include "edcas_post.hlsl"

#ifndef BLUR_VERTICAL
#define BLUR_VERTICAL 0
#endif

// the picture before the blur, for the blend (vertical pass)
Texture2D<float4> Original : register(t111);

float4 main(float4 pos : SV_Position) : SV_Target0
{
	int2 p = int2(pos.xy);
	int2 step = BLUR_VERTICAL ? int2(0, 1) : int2(1, 0);
	const float sigma = 3.0;
	float3 sum = PictureLoad(p);
	float total = 1;
	[unroll] for (int i = 1; i <= 12; i++)
	{
		float w = exp(-0.5 * i * i / (sigma * sigma));
		sum += w * (PictureLoad(p + i * step) + PictureLoad(p - i * step));
		total += 2 * w;
	}
	float3 blur = sum / total;
#if BLUR_VERTICAL
	float4 original = Original.Load(int3(pos.xy, 0));
	return float4(saturate(lerp(original.rgb, blur, saturate(0.5 * PostRow(601).z))), original.a);
#else
	return float4(blur, 1);
#endif
}
