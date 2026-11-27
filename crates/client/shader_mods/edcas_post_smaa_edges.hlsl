// SMAA's first pass (edcas_smaa.hlsl): the picture's edges into ResourceEdcasSmaaEdges (R8G8, cleared before;
// a pixel without an edge is discarded).
#include "edcas_smaa.hlsl"

float4 main(float4 pos : SV_Position) : SV_Target0
{
	float2 uv = SmaaSetUp(pos);
	float4 offset[3];
	SMAAEdgeDetectionVS(uv, offset);
	return float4(SMAAColorEdgeDetectionPS(uv, offset, Picture), 0, 0);
}
