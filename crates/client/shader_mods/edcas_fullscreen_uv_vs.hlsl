// One triangle over the whole target with the target's uv (0..1, y down) in TEXCOORD0, for passes that draw at
// another size than the picture they read (edcas_post_lenslight.hlsl, at a quarter). The position last: DXVK links
// the stages by register (edcas_sky_vs.hlsl, 2026-11-23).
void main(uint vertex : SV_VertexID, out float2 uv : TEXCOORD0, out float4 position : SV_Position)
{
	uv = float2((vertex << 1) & 2, vertex & 2);
	position = float4(uv * float2(2, -2) + float2(-1, 1), 0.5, 1);
}
