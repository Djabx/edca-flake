// The lowest and the highest ground in view for the fog (edcas.ini [CustomShaderEdcasGroundRef]):
// one texel, the lowest and highest height in the probe's buffer (edcas_ground_grid.hlsl), blended
// in by a small share each frame (ONE, INV_SRC_ALPHA; edcas_memory.hlsl: a mean over 2 s). x holds the lowest heights so weighted, z
// the highest, y the weights: x / y and z / y are their means, right from the first frames. So the
// fog's level follows the land over a few seconds and does not jump with every turn of the head.
// A second texel beside it (client 0.43.176) keeps the level of the terrain nearest the camera in the
// probe, the ground the camera is over as near as the picture tells, the same way over 1 s: x its
// height so weighted, y the weights, z its distance (metres). The dust takes its "seen from above"
// from it (edcas_dust.hlsl): over the lowest ground in view a camera on a hill counted as high up.
// A third texel (client 0.43.216) keeps how much of the terrain within SHADE_NEAR_M of the camera is in
// the sun by Elite's shadow (the probe's w), over 2 s: x the share so weighted, y the weights. The fog
// is lit by that much of the sun (edcas_fog.hlsl). It is the share of the probe's texels, so of the
// picture: the land looked at counts, and the nearer land more.

// The terrain that counts as round the camera, metres: as far as the fog is at its full (FOG_NEAR_M).
#define SHADE_NEAR_M 5000.0

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
	float shade = 0;
	int shadeSeen = 0;
	for (uint y = 0; y < height; y++)
		for (uint x = 0; x < width; x++) {
			float4 g = Grid.Load(int3(x, y, 0));
			if (g.z > 0 && g.z <= SHADE_NEAR_M && g.w >= 1) {
				shade += g.w - 1;
				shadeSeen++;
			}
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
	// Which of the three texels the pass is run into, by the place across them (-1 to 1).
	float place = v0.x / v0.z;
	// The third: the sun's share on the terrain round the camera.
	if (place > 1.0 / 3) {
		if (shadeSeen == 0)
			discard;  // no terrain that near in view, or no shadow to read: keep what there is
		float shadeShare = memoryShare(IniParams.Load(int2(525, 0)).y, 2.0);
		o0 = float4(shade / shadeSeen * shadeShare, shadeShare, 0, shadeShare);
		return;
	}
	// The second: the nearest terrain.
	if (place > -1.0 / 3) {
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
