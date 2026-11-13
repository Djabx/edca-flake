// The cockpit's light for the canopy frost (edcas_canopy_ice.hlsl): the picture before the glass
// (edcas.ini copies it once a frame), shrunk to a sixteenth, each texel the mean of its 16 x 16
// block (8 x 8 points). Read at single pixels the small bright lamps of the cockpit's light strings
// came out as hard, blocky specks in the frost (2026-10-28); shrunk first, they become soft light.
Texture2D<float4> Scene : register(t117);

void main(float4 position : SV_Position, out float4 o0 : SV_Target0)
{
	uint width, height;
	Scene.GetDimensions(width, height);
	int2 last = int2(width - 1, height - 1);
	int2 corner = int2(position.xy) * 16;
	float3 sum = 0;
	[unroll] for (int y = 0; y < 8; y++)
		[unroll] for (int x = 0; x < 8; x++)
			sum += Scene.Load(int3(min(corner + int2(x, y) * 2 + 1, last), 0)).rgb;
	o0 = float4(sum / 64, 1);
}
