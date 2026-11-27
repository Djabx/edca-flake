// SMAA's second pass (edcas_smaa.hlsl): the blending weights from the edges (t110 here, in the place of the
// picture) and SMAA's lookup textures (smaa_textures.py), into ResourceEdcasSmaaWeights (RGBA8, cleared before).
#include "edcas_smaa.hlsl"

Texture2D<float4> AreaTex : register(t111);
Texture2D<float4> SearchTex : register(t112);

float4 main(float4 pos : SV_Position) : SV_Target0
{
	float2 uv = SmaaSetUp(pos);
	float2 pixcoord;
	float4 offset[3];
	SMAABlendingWeightCalculationVS(uv, pixcoord, offset);
	return SMAABlendingWeightCalculationPS(uv, pixcoord, offset, Picture, AreaTex, SearchTex, float4(0, 0, 0, 0));
}
