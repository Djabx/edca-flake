// How much cloud or fog the ship is in, for the ice on the canopy (edcas_canopy_ice.hlsl): one
// texel, blended in by a share each frame like edcas_ground_ref.hlsl (x / y is the mean), so it
// rises and falls over about a fifth of a second. Built as edcas_canopy_frost.hlsl (CANOPY_FROST)
// it follows over about 5 s instead, the frost on the glass, and z / y is the light at the ship. Runs in the frame's cloud pass chain
// (edcas.ini), with the sky pass's constants: the camera is cb1[3], in atmosphere radii.
#define CLOUDS_MAIN cloudsMain
#include "edcas_clouds.hlsl"
#include "edcas_fog.hlsl"
#include "edcas_memory.hlsl"

// The slab's density at a point, scaled by its opacity (0 without a slab).
float cloudAt(float3 p)
{
	float sigma;
	return slabAt(p, sigma) * saturate(IniParams.Load(int2(504, 0)).x * 3);
}

void main(float3 v0 : TEXCOORD0, float3 v1 : TEXCOORD1, float3 v2 : TEXCOORD2, float3 v3 : TEXCOORD3,
          out float4 o0 : SV_Target0)
{
	float3 cam = cb1[3].xyz;
	float contact = saturate(cloudAt(cam) + fogAt(cam));
	// Sunlight at the ship, as the clouds get it, so the frost is not white at night.
	float r = length(cam);
	float rPlanet = cb1[31].z / cb1[31].w;
	float sky;
	float light = sunlight(dot(cam / r, -cb1[4].xyz), rPlanet / r, sky);
	// Seconds of memory: 5 for the frost, 0.09 for the contact (0.0015 and 0.08 a frame at 130 FPS before
	// 0.43.146; the 0.08 was noted as "about 0.2 s", which is where it has settled, two memories long).
#ifdef CANOPY_FROST
	float seconds = 5.0;
#else
	float seconds = 0.09;
#endif
	float share = memoryShare(IniParams.Load(int2(525, 0)).y, seconds);
	o0 = float4(contact * share, share, light * share, share);
}
