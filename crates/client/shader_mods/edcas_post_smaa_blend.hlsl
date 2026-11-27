// SMAA's third pass (edcas_smaa.hlsl): the picture blended with its neighbours by the weights, onto Elite's
// tonemap target.
#include "edcas_smaa.hlsl"

Texture2D<float4> Weights : register(t111);

float4 main(float4 pos : SV_Position) : SV_Target0
{
	float2 uv = SmaaSetUp(pos);
	float4 offset;
	SMAANeighborhoodBlendingVS(uv, offset);
	float4 c = SMAANeighborhoodBlendingPS(uv, offset, Picture, Weights);
	return float4(c.rgb, Picture.Load(int3(pos.xy, 0)).a);
}
