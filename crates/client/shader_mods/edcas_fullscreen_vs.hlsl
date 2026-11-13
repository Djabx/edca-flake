// One triangle over the whole target (draw = 3, 0), for passes of ours that are not drawn on the
// geometry of one of Elite's draws.
void main(uint vertex : SV_VertexID, out float4 position : SV_Position)
{
	float2 uv = float2((vertex << 1) & 2, vertex & 2);
	position = float4(uv * float2(2, -2) + float2(-1, 1), 0.5, 1);
}
