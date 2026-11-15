// The sun's direction some seconds ago, for the fog's dawn (edcas_fog.hlsl, edcas.ini
// [CustomShaderEdcasSunRef]): one texel, the direction towards the sun in the sky pass's planet frame
// (-cb1[4]; its axes are the world's, planet-fixed: cb1[19..21] are the identity over the atmosphere's
// radius, renderdoc/heightprobe.py, 2026-11-11) blended in by a small share each frame (ONE,
// INV_SRC_ALPHA), as edcas_ground_ref.hlsl keeps the ground: xyz / w is the mean over the last
// SUN_MEMORY_S seconds (edcas_memory.hlsl), 25 s. The sun turns in that frame as the planet
// turns, so the sun now less this mean says which way it moves: towards a point's zenith where the
// morning is (CLOUDS-PLAN.md phase 4). 6 e (a 79 h day) moves it by 5e-4 over 25 s, a 30-day day by
// 6e-5; the accumulator's rounding is some 2e-6.

cbuffer cb1 : register(b1)
{
	float4 cb1[334];
}

Texture1D<float4> IniParams : register(t120);

#include "edcas_memory.hlsl"

// The memory's length, seconds (0.0003 a frame at 130 FPS before 0.43.146). edcas_fog.hlsl's sunTrend
// compares the sun now with this mean against thresholds that assume it: change both or neither.
#define SUN_MEMORY_S 25.0

void main(float3 v0 : TEXCOORD0, float3 v1 : TEXCOORD1, float3 v2 : TEXCOORD2, float3 v3 : TEXCOORD3,
          out float4 o0 : SV_Target0)
{
	float3 sun = -cb1[4].xyz;
	if (!(dot(sun, sun) > 0.25))
		discard;  // no sun in the constants: keep what there is
	float share = memoryShare(IniParams.Load(int2(525, 0)).y, SUN_MEMORY_S);
	o0 = float4(normalize(sun) * share, share);
}
