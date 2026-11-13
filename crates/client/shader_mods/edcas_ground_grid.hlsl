// The ground probe (edcas.ini [CustomShaderEdcasGroundGrid]): run into a 32 x 18 buffer over
// the whole picture, each texel the height of the ground it sees within 300 m to 3 km (a share of the atmosphere's
// depth over the planet's radius, so that a value kept from another planet is not far off) and 1,
// or 0 where it sees no ground. edcas_ground_ref.hlsl keeps
// the lowest of them for the fog (edcas_fog.hlsl).
#define CLOUDS_MAIN cloudsMain
#include "edcas_clouds.hlsl"

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
	// its lamps lit none (2026-10-28). With no ground that close in view the level stays as it was
	// (edcas_ground_ref.hlsl keeps it).
	float rPlanet = cb1[31].z / cb1[31].w;
	float metres = ground * radiusM / rPlanet;
	if (metres < 300 || metres > 3000)
		return;
	o0 = float4((length(cb1[3].xyz + dir * ground) - rPlanet) / (1 - rPlanet), 1, 0, 0);
}
