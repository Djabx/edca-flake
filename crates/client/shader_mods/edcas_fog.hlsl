// Ground fog, for the frame's one cloud pass (edcas_clouds_final.hlsl). Included after
// edcas_clouds.hlsl, whose declarations and helpers it uses.
//
// Thin cold air fogs up overnight where the ground is lowest: on Mars the canyons of Noctis
// Labyrinthus fill with fog at sunrise and the plateaus between them stay clear (Viking), and at
// the Phoenix site the morning fog grew about 800 m deep, then burned off after sunrise. So the
// fog here is a layer up to a level, y507 metres over the lowest ground in view (edcas_ground_ref.hlsl
// keeps that height, smoothed over a few seconds), densest at the bottom, with a soft top a little
// uneven across the land. It is there at night and while the sun is low, and thins out as the sun climbs.
// The depth is bounded by half the relief in view (the highest ground seen over the lowest, from
// the same probe): fog that pools in the low ground needs higher ground to stay clear of. With
// the full y507 over any plain, the whole plain to the horizon lay under the fog's top, and seen
// from 10 km up the planet looked fogged all over, a white wall towards the horizon, with a haze
// everywhere on the ground too (Pro Thaa GO-T c3-5 4 a, 2026-10-30). A plain of 100 m relief
// now gets a 50 m sheet in its dips; a 2 km canyon the full depth, its plateaus clear.
// The fog is local: full to FOG_NEAR_M from the camera and gone by FOG_FAR_M. Its floor is the
// ground within 3 km of the camera, which says nothing about the land 40 km out; drawn that far
// anyway, the layer's own limb stood as a bright band above the horizon from a few hundred
// metres up, its top's bumps as beads along it (Pro Thaa GO-T c3-5 4 a, 2026-10-30).
//
// IniParams row 7: x507 density (0 = no fog; 1 = about 400 m of sight inside it; 0.1 looked right on
// 6 e), y507 depth in metres over the lowest ground (0 = 800, as at the Phoenix site; 400 looked too
// shallow on 6 e), z507 brightness (0 = 0.6), w507 the planet's radius in metres (from EDCAS; the fog
// needs it for its metres).

Texture2D<float4> GroundRef : register(t111);

#define FOG_STEPS 16
#define FOG_NEAR_M 5000.0
#define FOG_FAR_M 15000.0
// More steps where the ray spans more height, up to this many (see groundFog).
#define FOG_STEPS_MAX 48
// The fog's shape, from the IniParams and the lowest ground in view, in atmosphere radii.
struct Fog {
	float strength, radiusM, rPlanet, depthN, floorN, level, soft;
};

// False when there is no fog (x507 or the radius w507 not set, or no ground seen yet).
bool loadFog(out Fog f)
{
	float4 p7 = IniParams.Load(int2(507, 0));
	float4 ref = GroundRef.Load(int3(0, 0, 0));
	f.strength = p7.x;
	f.radiusM = p7.w;
	f.rPlanet = cb1[31].z / cb1[31].w;
	f.depthN = (p7.y > 0 ? p7.y : 800) * f.rPlanet / max(f.radiusM, 1);
	f.floorN = f.rPlanet + (ref.y > 0 ? ref.x / ref.y : 0) * (1 - f.rPlanet);
	float reliefN = (ref.y > 0 ? max(ref.z - ref.x, 0) / ref.y : 0) * (1 - f.rPlanet);
	f.depthN = min(f.depthN, 0.5 * reliefN);
	f.level = f.floorN + f.depthN;
	f.soft = 0.3 * f.depthN;
	// None from high up: there the depth buffer often has no ground, the march ran on through
	// the land, and the planet looked fogged all over and flickered as Elite switched its depth
	// buffers (approach to Pro Thaa VV-Z b2 1 d, 2026-10-28).
	float altitudeM = (length(cb1[3].xyz) - f.floorN) * f.radiusM / f.rPlanet;
	f.strength *= 1 - smoothstep(10000, 25000, altitudeM);
	return f.strength > 0 && f.radiusM > 0 && ref.y > 0 && f.depthN > 0;
}

// The sine of the sun's height at which the fog is gone (fogDensity's day factor is exactly 0 from it on).
#define FOG_DAY_SINE 0.35

// Whether any point within `metres` of the camera can hold fog: false when the sun is above
// FOG_DAY_SINE everywhere there, the day factor of fogDensity 0 at every sample. A point at a
// distance D of the camera, both at least rLow from the planet's centre (the fog's floor less its
// depth: below that fogDensity is 0 too), lies within 2 asin(D / (2 rLow)) of the camera's up seen
// from the centre, and the sun's height there differs by at most that angle. By day this spares
// the fog's two noises at each of its 16 to 48 samples a pixel, the lamp glow's and the canopy's,
// all multiplied by 0 before (PERFORMANCE.md C2, 2026-11-10).
bool fogWithin(Fog f, float metres)
{
	float3 cam = cb1[3].xyz;
	float3 sun = -cb1[4].xyz;
	float sunLength = length(sun);
	float rLow = max(f.floorN - f.depthN, 1e-6);
	float reach = metres * f.rPlanet / max(f.radiusM, 1) / (2 * rLow);
	if (!(sunLength > 0) || !(reach < 1))
		return true;
	// The camera's sun angle from the zenith, and the angle at which the day factor ends (with
	// 0.01 radians to spare, for the rounding of both).
	float zenith = acos(clamp(dot(cam, sun) * rsqrt(dot(cam, cam)) / sunLength, -1, 1));
	float clear = acos(min(FOG_DAY_SINE / sunLength, 1));
	return zenith + 2 * asin(reach) + 0.01 > clear;
}

// The fog's density at a point, 0..1 (the strength x507 is not in it).
float fogDensity(Fog f, float3 p)
{
	float r = length(p);
	float3 up = p / r;
	// The factors that need no noise first (night and a low sun, nothing below the floor, both
	// below): where one is 0 the density is, and the top's two noises are not worked out
	// (PERFORMANCE.md C3, 2026-11-10).
	float floorFade = smoothstep(f.floorN - f.depthN, f.floorN - 0.5 * f.depthN, r);
	float dayFade = 1 - smoothstep(0.1, FOG_DAY_SINE, dot(up, -cb1[4].xyz));
	if (floorFade == 0 || dayFade == 0)
		return 0;
	// The top rises and falls by half the depth over a few kilometres, and by a quarter of it
	// over some hundreds of metres: with the large bumps alone, seen side-on from just above
	// the layer (a sea of fog from a ridge) they averaged out and the top was a dead straight
	// line (2026-10-30).
	float top = f.level + (noise(up * (f.radiusM / 3000)) - 0.5) * f.depthN
	                    + (noise(up * (f.radiusM / 500) + 7.3) - 0.5) * 0.5 * f.depthN;
	float density = r < top ? 1 : exp(-(r - top) / f.soft);
	// Densest at the ground, a third of that at the top: evenly dense up to the top, the fog
	// was a white wall from inside (a long way through it sideways) and faint from above
	// (2026-10-27).
	density *= 0.3 + 0.7 * exp(-max(r - f.floorN, 0) / (0.4 * f.depthN));
	// Nothing below the floor: from space the depth buffer often has no ground for a ray (it
	// lies past the sky pass's own surface), and the march ran on into the planet, full of fog
	// there; the fog blinked as Elite switched between its depth buffers (2026-10-27).
	density *= floorFade;
	// Night and a low sun: gone by the time the sun is 20 degrees up.
	return density * dayFade;
}

// The fog along a pixel's view ray, premultiplied. inFront: the camera is in or under the fog's
// top, so the fog lies in front of the clouds.
float4 groundFog(float3 v0, float3 v1, out bool inFront)
{
	inFront = false;
	Fog f;
	if (!loadFog(f) || !fogWithin(f, FOG_FAR_M))
		return 0;
	float upper = f.level + 0.5 * f.depthN + 4 * f.soft;

	float3 dir;
	int2 pixel;
	float ground = viewRay(v0, v1, dir, pixel);
	float3 cam = cb1[3].xyz;
	inFront = dot(cam, cam) < upper * upper;

	// The stretch of the ray inside the fog's top sphere, up to the ground.
	float b = dot(cam, dir);
	float disc = b * b - (dot(cam, cam) - upper * upper);
	if (disc <= 0)
		return 0;
	float s = sqrt(disc);
	float t0 = max(-b - s, 0), t1 = -b + s;
	if (ground >= 0)
		t1 = min(t1, ground);
	// Beyond eight times the sight inside the fog nothing more shows through, and beyond
	// FOG_FAR_M there is none (see the top).
	float metresPerUnit = f.radiusM / f.rPlanet;
	float sigma = f.strength * 0.01 * metresPerUnit;  // 1 / m at density 1, in atmosphere radii
	t1 = min(t1, min(t0 + 8 / sigma, FOG_FAR_M / metresPerUnit));
	if (t1 <= t0)
		return 0;

	float3 sun = -cb1[4].xyz;
	float3 sunColour = starLightTint() * cb1[8].xyz / max(max(cb1[8].x, cb1[8].y), max(cb1[8].z, 1e-30));
	float brightness = IniParams.Load(int2(507, 0)).z;
	brightness = brightness > 0 ? brightness : 0.6;
	// Lit by the sun and the sky only: in the dark the fog is dark and hides the
	// land behind it. An ambient light like the one Elite keeps the land visible with at night made
	// it glow grey, as if it shone by itself, even at a fifth (Pro Thaa VV-Z b2 1 d, 2026-10-28).
	// The steps are even in height, not in distance: the density is a profile over the height
	// (steep, its scale a third of the depth) that hardly varies along the land, and towards the
	// horizon a ray runs tens of kilometres through a layer a few tens of metres deep. Even steps
	// along it, jittered per pixel, sampled that profile at heights that differed from pixel to
	// pixel, and the fog along the horizon was grain (2026-10-30). The ray's height along it,
	// r(t)^2 = |cam|^2 + 2 b t + t^2, falls to its lowest at t = -b and rises again: each of the
	// two parts gets steps even in r, as many as its share of the height spanned. A step spans
	// half the profile's scale at most (the top's fall-off, `soft`): at 16 steps over the whole
	// shell a step was over a scale, the fall-off changed by e between samples, and the fog
	// along the horizon showed the steps as lines (2026-10-30).
	float cc = dot(cam, cam);
	float rmin2 = max(cc - b * b, 0);
	float tm = clamp(-b, t0, t1);
	float rA = sqrt(cc + t0 * (2 * b + t0)), rM = sqrt(cc + tm * (2 * b + tm)), rB = sqrt(cc + t1 * (2 * b + t1));
	float spanA = abs(rA - rM), spanB = abs(rB - rM);
	int steps = clamp((int)ceil((spanA + spanB) / (0.5 * f.soft)), FOG_STEPS, FOG_STEPS_MAX);
	int nA = (int)round(steps * spanA / max(spanA + spanB, 1e-30));
	float3 col = 0;
	float T = 1;
	float tPrev = t0;
	[loop] for (int i = 0; i < steps; i++) {
		bool down = i < nA;
		float k = down ? (i + 1) / (float)nA : (i + 1 - nA) / (float)(steps - nA);
		float rEnd = down ? lerp(rA, rM, k) : lerp(rM, rB, k);
		float half = sqrt(max(rEnd * rEnd - rmin2, 0));
		float tEnd = clamp(down ? -b - half : -b + half, tPrev, t1);
		float dt = tEnd - tPrev;
		float3 p = cam + dir * (0.5 * (tPrev + tEnd));
		tPrev = tEnd;
		if (dt <= 0)
			continue;
		float density = fogDensity(f, p) * (1 - smoothstep(FOG_NEAR_M, FOG_FAR_M, 0.5 * (tPrev + tEnd) * metresPerUnit));
		if (density <= 0.001)
			continue;
		float r = length(p);
		float day = dot(p / r, sun);
		float sky;
		float light = sunlight(day, f.rPlanet / r, sky);
		float3 tint = lerp(float3(1.0, 0.6, 0.45), float3(1, 1, 1), saturate(day * 3));
		float3 radiance = sunColour * brightness * (0.8 * light + 0.25 * sky) * tint;
		float a = 1 - exp(-sigma * density * dt);
		col += T * a * radiance;
		T *= 1 - a;
		if (T < 0.01)
			break;
	}
	return float4(col, 1 - T);
}

// The fog's density at a point times its strength x507, for the ice on the canopy
// (edcas_canopy_contact.hlsl).
float fogAt(float3 p)
{
	Fog f;
	return loadFog(f) && fogWithin(f, length(p - cb1[3].xyz) * f.radiusM / f.rPlanet) ? fogDensity(f, p) * saturate(f.strength * 10) : 0;
}
