// Value noise for the EDCAS shaders (edcas_clouds.hlsl, edcas_canopy_ice.hlsl): a hash per
// lattice point, smoothly interpolated.

float hash(float3 p)
{
	p = frac(p * 0.3183099 + 0.1);
	p *= 17.0;
	return frac(p.x * p.y * p.z * (p.x + p.y + p.z));
}

float noise(float3 x)
{
	float3 i = floor(x);
	float3 f = frac(x);
	f = f * f * (3.0 - 2.0 * f);
	return lerp(lerp(lerp(hash(i + float3(0, 0, 0)), hash(i + float3(1, 0, 0)), f.x),
	                 lerp(hash(i + float3(0, 1, 0)), hash(i + float3(1, 1, 0)), f.x), f.y),
	            lerp(lerp(hash(i + float3(0, 0, 1)), hash(i + float3(1, 0, 1)), f.x),
	                 lerp(hash(i + float3(0, 1, 1)), hash(i + float3(1, 1, 1)), f.x), f.y), f.z);
}

// The colour of the star's light EDCAS sets (star_light.rs): IniParams row 526, x y z the star's colour
// at luminance 1, w how far towards it (0 = Elite's own, nearly white, light); x527 blends it into the
// witch-space rainbow, the hue turning on z515's clock (the client writes 1 during a hyperspace jump
// with the next star's colour; 3DMigoto's fade slides into the rainbow and out into that colour).
// Multiplies a sun colour.
float3 starLightTint()
{
	float4 p = IniParams.Load(int2(526, 0));
	float3 tint = 1 + (p.xyz - 1) * max(p.w, 0);
	float t = IniParams.Load(int2(515, 0)).z;
	float3 rainbow = 0.3 + 0.1 * cos(6.283185 * (frac(t * 0.3) + float3(0, 1.0 / 3, 2.0 / 3)));
	return lerp(tint, rainbow, saturate(IniParams.Load(int2(527, 0)).x));
}
