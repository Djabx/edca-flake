// The lowest and the highest ground in view for the fog (edcas.ini [CustomShaderEdcasGroundRef]):
// one texel, the lowest and highest height in the probe's buffer (edcas_ground_grid.hlsl), blended
// in by a small share each frame (ONE, INV_SRC_ALPHA). x holds the lowest heights so weighted, z
// the highest, y the weights: x / y and z / y are their means, right from the first frames. So the
// fog's level follows the land over a few seconds and does not jump with every turn of the head.

Texture2D<float4> Grid : register(t111);

void main(float3 v0 : TEXCOORD0, float3 v1 : TEXCOORD1, float3 v2 : TEXCOORD2, float3 v3 : TEXCOORD3,
          out float4 o0 : SV_Target0)
{
	uint width, height;
	Grid.GetDimensions(width, height);
	float lowest = 1e30, highest = -1e30;
	int seen = 0;
	for (uint y = 0; y < height; y++)
		for (uint x = 0; x < width; x++) {
			float2 g = Grid.Load(int3(x, y, 0)).xy;
			if (g.y > 0) {
				lowest = min(lowest, g.x);
				highest = max(highest, g.x);
				seen++;
			}
		}
	if (seen < 4)
		discard;  // only sky in view: keep what there is
	float share = 0.004;  // about 2 s at 130 FPS
	o0 = float4(lowest * share, share, highest * share, share);
}
