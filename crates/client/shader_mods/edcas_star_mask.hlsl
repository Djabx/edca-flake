// The star mask (README.md "Post effects", 2026-11-26): 1 where Elite's star surface shader 9649b8374f5c9aeb draws,
// for the lens effects, which only a star lights (edcas_post_lenslight.hlsl). Drawn right after each of that
// shader's draws with Elite's own vertex shader and state (edcas.ini: draw = from_caller) into
// ResourceEdcasStarMask. Elite's shader tests the depth itself, by its first seven instructions, repeated here:
// the scene's depth (t0, its sampler s2) at the pixel against the star's own (v0.z), and what stands before the
// star (the cockpit's frame, a planet) is discarded. In ed_frame27784 that draw's pixels are the sun's disc with
// the canopy's frame and the dashboard's plants cut out (renderdoc/drawmask.py).
Texture2D<float4> Depth : register(t0);
SamplerState DepthSampler : register(s2);

float4 main(float3 v0 : TEXCOORD0) : SV_Target0
{
	float2 uv = v0.xy / v0.z * 0.5 + 0.5;
	float depth = Depth.Sample(DepthSampler, float2(uv.x, 1 - uv.y)).x;
	if (depth - v0.z < 0)
		discard;
	return 1;
}
