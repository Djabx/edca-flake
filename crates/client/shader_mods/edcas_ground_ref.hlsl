// The lowest and the highest ground in view for the fog (edcas.ini [CustomShaderEdcasGroundRef]):
// one texel, the lowest and highest height in the probe's buffer (edcas_ground_grid.hlsl), blended
// in by a small share each frame (ONE, INV_SRC_ALPHA; edcas_memory.hlsl: a mean over 2 s). x holds the lowest heights so weighted, z
// the highest, y the weights: x / y and z / y are their means, right from the first frames. So the
// fog's level follows the land over a few seconds and does not jump with every turn of the head.
// A second texel beside it (client 0.43.176) keeps the level of the terrain nearest the camera in the
// probe, the ground the camera is over as near as the picture tells, the same way over 1 s: x its
// height so weighted, y the weights, z its distance (metres). The dust takes its "seen from above"
// from it (edcas_dust.hlsl): over the lowest ground in view a camera on a hill counted as high up.

Texture2D<float4> Grid : register(t111);
Texture1D<float4> IniParams : register(t120);

#include "edcas_memory.hlsl"

void main(float3 v0 : TEXCOORD0, float3 v1 : TEXCOORD1, float3 v2 : TEXCOORD2, float3 v3 : TEXCOORD3,
          out float4 o0 : SV_Target0)
{
	uint width, height;
	Grid.GetDimensions(width, height);
	float lowest = 1e30, highest = -1e30, farLowest = 1e30, farHighest = -1e30;
	float nearest = 1e30, nearestHeight = 0;
	int seen = 0, farSeen = 0;
	for (uint y = 0; y < height; y++)
		for (uint x = 0; x < width; x++) {
			float3 g = Grid.Load(int3(x, y, 0)).xyz;
			if (g.z > 0 && g.z < nearest) {
				nearest = g.z;
				nearestHeight = g.x;
			}
			if (g.y > 0.5) {          // ground 300 m to 3 km off: the valley at hand
				lowest = min(lowest, g.x);
				highest = max(highest, g.x);
				seen++;
			} else if (g.y > 0.1) {   // ground farther off (edcas_ground_grid.hlsl)
				farLowest = min(farLowest, g.x);
				farHighest = max(farHighest, g.x);
				farSeen++;
			}
		}
	// The second texel (the right half of the two the pass is run into): the nearest terrain.
	if (v0.x / v0.z > 0) {
		if (nearest > 1e29)
			discard;  // no terrain in view: keep what there is
		float nearShare = memoryShare(IniParams.Load(int2(525, 0)).y, 1.0);
		o0 = float4(nearestHeight * nearShare, nearShare, nearest * nearShare, nearShare);
		return;
	}
	// With none of the valley in view (high over the land, or its near ground hidden by the cockpit)
	// the ground farther off gives the level: kept as it was, it was another place's (client 0.43.152,
	// edcas_ground_grid.hlsl).
	if (seen < 4) {
		lowest = farLowest;
		highest = farHighest;
		seen = farSeen;
	}
	if (seen < 4)
		discard;  // only sky in view: keep what there is
	float share = memoryShare(IniParams.Load(int2(525, 0)).y, 2.0);  // 0.004 a frame at 130 FPS before 0.43.146
	o0 = float4(lowest * share, share, highest * share, share);
}
