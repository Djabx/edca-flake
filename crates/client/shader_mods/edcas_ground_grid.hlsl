// The ground probe (edcas.ini [CustomShaderEdcasGroundGrid]): run into a 32 x 18 buffer over
// the whole picture, each texel the height of what it sees (x: a share of the atmosphere's depth over
// the planet's radius, so that a value kept from another planet is not far off) and its kind (y): 1 for
// ground within 300 m to 3 km, 0.25 from 3 km to GROUND_FAR_M, 0 for neither.
// edcas_ground_ref.hlsl keeps the lowest of them for the fog (edcas_fog.hlsl) and the dust
// (edcas_dust.hlsl): of the first kind, and of the second only while none of the first is in view.
// z (client 0.43.176): how far off it is, metres, where it is terrain at any distance (the depth
// buffer's as far as the terrain mask's, as edcas_dust.hlsl tells ground from a hull), else 0:
// edcas_ground_ref.hlsl keeps the level of the nearest, the ground the camera is over, for the dust.
// w (client 0.43.216): 1 + the sun's share on it by Elite's shadow (sunShade, edcas_clouds.hlsl), so 1 in
// shadow and 2 in the sun, or 0 where that is not to be had: edcas_ground_ref.hlsl keeps its mean over
// the terrain round the camera, for the fog's light (edcas_fog.hlsl).

// How far out ground still gives a level where none is nearer: the fog and the dust's far drift are
// gone by 15 km.
#define GROUND_FAR_M 20000.0
#define CLOUDS_MAIN cloudsMain
#include "edcas_clouds.hlsl"

Texture2D<float> TerrainDist : register(t118);  // the terrain mask (edcas.ini [ResourceEdcasTerrainDist]), metres

void main(float3 v0 : TEXCOORD0, float3 v1 : TEXCOORD1, float3 v2 : TEXCOORD2, float3 v3 : TEXCOORD3,
          out float4 o0 : SV_Target0)
{
	o0 = 0;
	float radiusM = IniParams.Load(int2(507, 0)).w;
	float3 dir;
	int2 pixel;
	float ground = viewRay(v0, v1, dir, pixel);
	if (ground < 0 || radiusM <= 0)
		return;
	// The cockpit, the ship seen from outside and anything else this close is not ground. Nor does
	// ground further than 3 km count: fog pools in the valley at hand, and from a free camera high
	// up or far out the lowest ground in view lay in other valleys, the fog sank under the ship and
	// its lamps lit none (2026-10-28).
	// With no ground that close in view the level used to stay as it was (edcas_ground_ref.hlsl kept
	// it): but the ship moves on, and the level it took along was another place's. Three kilometres
	// over a basin's floor, the last ground within 3 km had been its rim, at the ship's own height:
	// the dust drew its billows and its far drift at the ship's flight level, a bright smear over the
	// hull, the sky washed out, and rings round the ship, which came and went within some tens of
	// metres of climb (ed_frame109427, ed_frame118853, ed_frame154231, 2026-11-12: the camera within
	// 24, 300 and 100 m of the kept level, the dust 54.8 of 255 over the picture in the last). And
	// where no level had ever been kept the dust's was the planet's own radius (ed_frame171605). So
	// ground farther off counts too, marked 0.25: edcas_ground_ref.hlsl takes it only while it sees
	// none of the near kind, so with the valley in view the level is as it was.
	float rPlanet = cb1[31].z / cb1[31].w;
	float metres = ground * radiusM / rPlanet;
	float mask = TerrainDist.Load(int3(pixel, 0));
	float terrain = mask > 0 && abs(mask - metres) < 0.2 + 0.02 * metres ? metres : 0;
	float kind = metres < 300 || metres > GROUND_FAR_M ? 0 : metres <= 3000 ? 1 : 0.25;
	o0 = float4((length(cb1[3].xyz + dir * ground) - rPlanet) / (1 - rPlanet), kind, terrain, 1 + sunShade(pixel));
}
