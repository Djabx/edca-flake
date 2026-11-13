// EDCAS cloud layer for the sky of landable atmospheric planets (3DMigoto custom shader).
//
// Where it runs. Elite draws the sky and the haze over the ground in one pass, pixel shader
// 236d6421041d3892 or one of its variants 7932556525410d85 and 8c9b2131f7ef329f (head look).
// edcas.ini runs this shader once a frame after Elite's last full-size draw of that pass, into a
// half-size buffer (edcas_clouds_final.hlsl, which adds the ground fog of edcas_fog.hlsl), and
// edcas_clouds_composite.hlsl lays that over the picture. In a frame without a full-size draw
// it runs after the small sky draw instead. It keeps
// the pass's vertex shader, constant buffers and textures, so its inputs are 236d6421's:
//   v0      clip position x, y, w (screen position and the depth of the pass's own surface)
//   v1      view ray, turned towards the planet by cb1[19..21]
//   cb1[3]  camera relative to the planet centre, in atmosphere radii
//   cb1[4]  direction the sunlight travels (negated: towards the sun)
//   cb1[8]  the sunlight the pass scatters (normalised; a global constant, not the star's colour, so the
//           clouds' colour is it times starLightTint(), IniParams row 526, edcas_noise.hlsl)
//   cb1[31] z = planet radius, w = atmosphere radius (same unit as each other)
//   cb1[25].w == 1: depth is in t4, else in t0
//
// What it draws. Sunlit ice in the sun's colour on the pass's exposed scale (1 = white): bright
// against a thin atmosphere's dark sky by day, lit a little after the ground goes dark, then dark
// against the afterglow and the stars. With z505 = 0 a single thin shell at height z503; with z505 > 0
// a thin slab around it, ray-marched (44 to 80 steps in the half-size pass) with self-shadowing, flat
// tops, soft bases and frayed edges, so you can fly through it. Clouds in thin air are veils in
// flat layers (Mars, Pluto): no towers, and the stars show through. The noise is squeezed along
// the wind into streaks, warped and eroded; near the sun the clouds turn pastel (iridescence).
//
// IniParams (edcas.ini [Constants], live from ShaderFixes/edcas_params.txt; 0 = the default):
//  Written by EDCAS per planet
//   x503 coverage 0..1 (0 = no clouds)     y503 clouds across the sky (larger = smaller)
//   z503 height 0..1 up to the ceiling     w503 brightness (1 = bright ice)
//   x504 opacity: the thin shell's largest alpha, or the slab's along a vertical path through it
//   y504 streakiness 0..1 (0 = puffy)      w504 iridescence 0..1
//   z505 slab thickness, a share of the ceiling (0 = the thin shell; 0.1 for most clouds)
//   x507, w507 ground fog: density and the planet's radius in metres (see edcas_fog.hlsl)
//  Look, tuned in game (2026-10-26/27)
//   y505 how far out the clouds are seen, in planet radii above the ground (20)
//   y506 the ceiling, a share of the atmosphere (1; lower ones lay inside Elite's mountains)
//   z506 flat tops: the share of the slab under its top where tops round off (0.35)
//   w506 how fast the clouds darken after sunset, the twilight curve's power (4)
//   x508 soft, frayed edges (0.5; larger = softer)
//   y508 how far up the bases fade in, a share of the slab (0.35)

// Steps along each ray in thick mode: at least MARCH_STEPS, up to MAX_STEPS along a long path
// (the march keeps a step under half a cloud feature, 2026-11-08). The half-size pass can afford
// more, which keeps its grain fine (170 FPS with 20 at 3440x1440, 150 with 32 and some grain
// left, 2026-10-26). The detail (octaves) follows a step of a 32nd of the ray, so more steps only
// smooth the grain.
#ifdef EDCAS_FINAL
#define MARCH_STEPS 44
#define MAX_STEPS 80
#else
#define MARCH_STEPS 20
#define MAX_STEPS 32
#endif

cbuffer cb1 : register(b1)
{
	float4 cb1[334];
}


Texture2D<float4> Depth : register(t0);
Texture2D<float4> DepthAlt : register(t4);
Texture1D<float4> IniParams : register(t120);

// Elite draws the atmosphere pass several times a frame, each draw over a band of view directions
// with a soft edge (cb1[25].w == 3; the weight below is the pass's own, from its first lines):
// the bands' weights add up to 1, so its sky comes out whole. The clouds, drawn in full after
// every one of those draws, came out twice where two bands overlap, with a hard line along a
// band's edge 2.4 degrees under the horizon (2026-10-26; a per-pixel mask could not keep them to
// one draw). With this weight w, each draw puts on alpha 1 - (1 - a)^w, and together the draws
// give exactly alpha a, in any order.
float bandWeight(float3 v1)
{
	if (asint(cb1[25].w) != 3)
		return 1;
	float x = 1 - abs((dot(normalize(v1), cb1[25].xyz) - cb1[24].z) / cb1[24].w);
	if (x <= 0)
		return 0;
	return x * x * (3 - 2 * x);
}

float inBand(float alpha, float w)
{
	return 1 - pow(saturate(1 - alpha), w);
}

#include "edcas_noise.hlsl"

// 0..1, n octaves (n is a constant at every call, so the loop unrolls)
float fbm(float3 p, const int n)
{
	float sum = 0, amp = 0.5, total = 0;
	[unroll] for (int i = 0; i < n; i++) {
		sum += amp * noise(p);
		total += amp;
		p = p * 2.03 + 17.1;
		amp *= 0.5;
	}
	return sum / total;
}

// fbm with up to 5 octaves, the last one faded in by the fraction: octaves finer than the march
// steps can resolve are left out (see the thick mode's march). n0, n1 are the first two octaves'
// noises, which the march's cheap skip has worked out already (`fbm(p, 2)`); an octave of weight 0
// is not worked out at all (2026-11-10, PERFORMANCE.md C4: it was, and multiplied by 0).
float fbmLod(float3 p, float octaves, float n0, float n1)
{
	float sum = 0, amp = 0.5, total = 0;
	[unroll] for (int i = 0; i < 5; i++) {
		float w = saturate(octaves - i);
		float n = i == 0 ? n0 : n1;
		if (i > 1) {
			n = 0;
			[branch] if (w > 0)
				n = noise(p);
		}
		sum += amp * w * n;
		total += amp * w;
		p = p * 2.03 + 17.1;
		amp *= 0.5;
	}
	return sum / total;
}

// How much sunlight still reaches a point with the sun at day (the sine of its height there),
// 1 with the sun a degree up or more, 0 where it sets behind the planet's edge (-limit). (Full
// only from 6 degrees up, the fog on the ground with the sun 3 degrees up got a sixteenth of
// the light and was barely seen, 2026-10-27.) The
// light just before that crosses the thickest air along the ground and is mostly gone: the
// clouds kept a third of full sun until then and glowed bright over the dark ground after
// sunset (2026-10-26). w506 = the curve's power (0 = 4; larger = darker sooner; 4 looked better
// in game than 2: dark clouds against the afterglow, a faint rim near the horizon).
float twilight(float day, float limit)
{
	float power = IniParams.Load(int2(506, 0)).w;
	return pow(smoothstep(-limit, 0.02, day), power > 0 ? power : 4);
}

// The sunlight at a point with the sun at day (the sine of its height there) and the planet's
// radius over the point's (both from its centre): the sun is seen while it is not lower than
// the point's own horizon, which lies below the ground's, so high clouds stay lit a little after
// the ground goes dark. sky: the share of the sky light left (dusk); the return value adds the
// sun's own height. Shared by the clouds, the fog and the canopy frost.
float sunlight(float day, float planetOverR, out float sky)
{
	float limit = sqrt(max(1 - planetOverR * planetOverR, 0));
	sky = smoothstep(-limit - 0.03, -limit + 0.03, day) * twilight(day, limit);
	return sky * max(saturate(day * 4 + 0.15), 0.35);
}

// ---- Thick mode (z505 > 0) ----

// 0..1 across the slab (h = 0 bottom, 1 top): fades the base in and stops the tops at one level,
// the slab's top (as under an inversion: gravity is the same all round the planet). Set by
// tops, the share of the slab under the top over which the tops round off (z506; 0 = 0.35). With
// 0.5 every cloud rounded off at its own height and from orbit they stuck out of the atmosphere
// with gaps between them along the planet's edge (criticism of the post, 2026-10-26); 0.15 made
// boxes with steep sides. A thin slab matters more: at z505 0.3 the clouds were as tall as wide and
// hung as curtains, z505 0.1 lies as flat patches and a smooth band at the edge (thin air has no
// towers: Mars's and Pluto's clouds are thin layers).
static float tops = 0.35;
// The base fades in over the lowest y508 of the slab (0 = 0.35). Over a tenth, the clouds' bases
// near the horizon were cut along one straight line (2026-10-27).
static float bases = 0.35;
// How softly a cloud's edge fades out (x508; 0 = 0.5): the noise's ramp from clear to full cloud is
// 0.15 + 0.3 x508 wide and the fine erosion bites 1 + x508 times half as hard. With the ramp 0.15 and
// half the erosion every cloud was a smooth blob with a clear outline, where clouds in thin air
// fray out (2026-10-27).
static float edges = 0.5;
float slabProfile(float h)
{
	return smoothstep(0, bases, h) * smoothstep(1, 1 - tops, h);
}

// The 3D noise coordinate at a point: the shell's coordinate (up * shell / layer * scale / 4)
// extended to any radius, squeezed along east (x) into streaks, then bent by the sample's warp.
// Only the point's spot on the shell is squeezed; its height above or below the shell is added
// along its own up afterwards. Squeezing the whole point along x tilted every cloud the same way
// wherever x is not level, which is most of the planet: all of them leaned like flames on the
// horizon seen from orbit (2026-10-26).
float3 cloudCoord(float3 p, float k, float shellN, float squeeze, float3 warp)
{
	float r = length(p);
	float3 up = p / r;
	float3 at = up * (shellN * k);
	at.x *= squeeze;
	return at + up * ((r - shellN) * k) + warp;
}

// Cloud density 0..1 at a noise coordinate and a height in the slab. The profile is subtracted
// from the noise before the threshold, so the tops round off and the base thins out.
float slabDensity(float3 at, float h, float threshold, const int octaves)
{
	float f = fbm(at, octaves) - 0.25 * (1 - slabProfile(h));
	return smoothstep(threshold - 0.03, threshold + 0.12 + 0.3 * edges, f);
}

float slabDensityLod(float3 at, float h, float threshold, float octaves, float n0, float n1)
{
	float f = fbmLod(at, octaves, n0, n1) - 0.25 * (1 - slabProfile(h));
	return smoothstep(threshold - 0.03, threshold + 0.12 + 0.3 * edges, f);
}

// The cheap density of the sun-shadow taps: two octaves at most, the second faded in by the
// octaves the tap's spacing can resolve (1..2), as fbmLod does for the march's samples. With
// two fixed octaves the taps sampled detail finer than their spacing at a high Thickness setting
// (their lengths scale with the slab, the clouds keep the planet's size), and each pixel's
// sunlight came out random: brightness grain growing with the setting (a tester, 2026-11-08).
float slabDensityShadow(float3 at, float h, float threshold, float octaves)
{
	float w = saturate(octaves - 1);
	float second = 0;
	[branch] if (w > 0)
		second = noise(at * 2.03 + 17.1);
	float f = (0.5 * noise(at) + 0.25 * w * second) / (0.5 + 0.25 * w) - 0.25 * (1 - slabProfile(h));
	return smoothstep(threshold - 0.03, threshold + 0.12 + 0.3 * edges, f);
}

// A pixel's offset of the march's samples along its ray (0..1), an ordered 4 x 4 pattern (Bayer),
// permuted per tile:
// the composite's tent over 4 x 4 texels of the half-size buffer sees every offset once, so the
// step banding cancels instead of averaging out as white noise would. On the capture
// ed_frame511315 (2026-11-08, the lower edges of the clouds over the haze grainy, a tester's
// report) it halved the error against a converged march (352 steps) at the same cost, 0.47 to
// 0.25 of 255 rms, as much as doubling the steps did, and the grain fell to the converged
// picture's own; a random offset (pcg3d) left the grain, a wider tent changed nothing.
float bayerJitter(uint2 p)
{
	const float m[16] = { 0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5 };
	// Every 4 x 4 tile permutes the sixteen offsets its own way (an XOR with a hash of the tile is
	// a bijection on 0..15, so each tile still holds every offset once): the plain tile's repeat
	// lined up into fine diagonal stripes across the cloud layer seen from high altitude with the
	// free camera zoomed out (2026-11-08, after 0.43.85). On ed_frame511315 the error against the
	// converged march is 0.32 with the permutation, 0.25 without, 0.47 with a random offset.
	uint2 tile = p >> 2;
	uint h = tile.x * 0x9e3779b1u ^ tile.y * 0x85ebca6bu;
	h ^= h >> 15; h *= 0x2c1b3c6du; h ^= h >> 12;
	uint i = ((p.y & 3) * 4 + (p.x & 3)) ^ (h & 15);
	return (m[i] + 0.5) / 16.0;
}

// The slab's geometry in atmosphere radii and its noise scale, from the IniParams (z503, y503, x503,
// y504, z505 and the ceiling y506); also sets the look the slab shares (z506, x508, y508 into tops, edges,
// bases). For main's thick mode and the canopy's cloudAt (edcas_canopy_contact.hlsl).
struct Slab {
	float rPlanet, shellN, thickN, bottom, top, span, k, squeeze, threshold;
};

Slab loadSlab()
{
	float4 params = IniParams.Load(int2(503, 0));
	float4 params6 = IniParams.Load(int2(506, 0));
	float4 params8 = IniParams.Load(int2(508, 0));
	if (params6.z > 0)
		tops = saturate(params6.z);
	if (params8.x > 0)
		edges = params8.x;
	if (params8.y > 0)
		bases = saturate(params8.y);
	// The clouds keep to the lowest y506 of the atmosphere (0 = 1, all of it): z503 = 1 is that
	// ceiling, and the slab's thickness (z505) is a share of it too. Lowering it did not work: at
	// 0.25 and at 0.5 the clouds on 6 e lay inside its mountains and were gone, from 15 km up and
	// looking straight down (2026-10-26). Elite's terrain is high for its atmosphere.
	float ceiling = params6.y > 0 ? params6.y : 1;
	Slab s;
	s.rPlanet = cb1[31].z / cb1[31].w;
	s.shellN = s.rPlanet + saturate(params.z) * ceiling * (1 - s.rPlanet);
	s.thickN = saturate(IniParams.Load(int2(505, 0)).z) * ceiling * (1 - s.rPlanet);
	// Bottom and top of the slab, centred on the shell. With z503 near 0 it sits on the ground,
	// which is meant: fog banks.
	s.bottom = max(s.rPlanet, s.shellN - s.thickN * 0.5);
	s.top = min(1, s.shellN + s.thickN * 0.5);
	s.span = max(s.top - s.bottom, 1e-5);
	// Noise in units of the layer's height above the ground, so the clouds look the same size on
	// every planet: y503 = 8 makes a cloud about half the height. (Units of the distance to the
	// layer's horizon made a low layer's clouds hundreds of km wide, 2026-10-26.)
	s.k = params.y / 4 / max(s.shellN - s.rPlanet, 1e-4 * s.rPlanet);
	s.squeeze = 1 / (1 + 6 * saturate(IniParams.Load(int2(504, 0)).y));
	// Threshold from the noise's spread (mean 0.5, sd 0.11 after the warp), so that about x503 of
	// the sky is cloud: before, coverage 0.25 (a sparse planet from EDCAS) left 1 %. In an offline
	// port of the slab (camera on the ground, z503 0.24, z505 0.3) x503 = 0.25 covered 23 % of the sky
	// above 10 degrees (2026-10-26).
	s.threshold = 0.5 + 0.3 * (0.5 - params.x);
	return s;
}

// The slab's density at a point, as the thick mode's march works it out (0 without a slab), and
// its extinction at density 1, per atmosphere radius (as the march's sigma). For the canopy's
// contact measure and the light of Elite's lamps in the cloud (edcas_canopy_contact.hlsl,
// edcas_lights.hlsl).
float slabAt(float3 p, out float sigma)
{
	sigma = 0;
	float4 params = IniParams.Load(int2(503, 0));
	float opacity = IniParams.Load(int2(504, 0)).x;
	if (params.x <= 0 || opacity <= 0 || IniParams.Load(int2(505, 0)).z <= 0)
		return 0;
	Slab slab = loadSlab();
	float r = length(p);
	if (r <= slab.bottom || r >= slab.top)
		return 0;
	sigma = -log(1 - min(opacity, 0.999)) / slab.span;
	float3 atPlain = cloudCoord(p, slab.k, slab.shellN, slab.squeeze, 0);
	float3 warp = (noise(atPlain * 0.5 + 3.7) - 0.5) * 1.6 * float3(1, -0.6, 0.8);
	return slabDensity(atPlain + warp, (r - slab.bottom) / slab.span, slab.threshold, 4);
}

// The view ray of a pixel in the planet's frame (atmosphere radii, the camera at cb1[3]) and how
// far along it the depth buffer has something drawn (the ground, a ship): -1 for nothing. As
// main works it out; for the fog and the ground probe (edcas_fog.hlsl, edcas_ground_grid.hlsl).
float viewRay(float3 v0, float3 v1, out float3 dir, out int2 pixel)
{
	float2 uv = v0.xy / v0.z * float2(0.5, -0.5) + 0.5;
	bool alt = asint(cb1[25].w) == 1;
	uint width, height;
	if (alt)
		DepthAlt.GetDimensions(width, height);
	else
		Depth.GetDimensions(width, height);
	pixel = int2(min(uv * float2(width, height), float2(width - 1, height - 1)));
	float depth = alt ? DepthAlt.Load(int3(pixel, 0)).x : Depth.Load(int3(pixel, 0)).x;
	dir = normalize(float3(dot(cb1[19].xyz, v1), dot(cb1[20].xyz, v1), dot(cb1[21].xyz, v1)));
	// The pass's own surface, the atmosphere's shell: the clip w of the dome's vertex, or, run over
	// the whole picture without the dome (edcas_sky_vs.hlsl gives v0.z < 0, 2026-11-08), the shell's
	// view depth worked out here: the ray's exit from the unit sphere round the planet from inside
	// it, its entry from outside, in metres by the rows' scale (atmosphere radii per metre), along
	// the view's forward axis. No hit (outside, looking past the planet) is no surface: all is sky.
	float own = v0.z;
	if (own < 0) {
		float3 c = cb1[3].xyz;
		float cd = dot(c, dir), cc = dot(c, c);
		float disc = cd * cd - cc + 1;
		float t = disc > 0 ? (cc < 1 ? -cd + sqrt(disc) : -cd - sqrt(disc)) : 0;
		own = t > 0 ? t / length(cb1[19].xyz) * dot(normalize(v1), cb1[279].xyz) : 0;
	}
	if (depth >= own)
		return -1;
	float4 view = float4(v1 / dot(v1, cb1[279].xyz) * depth, 1);
	float3 hit = float3(dot(cb1[19], view), dot(cb1[20], view), dot(cb1[21], view));
	return length(hit - cb1[3].xyz);
}

// The frame's one cloud pass (EDCAS_FINAL) runs this as a function and adds the fog
// (edcas_clouds_final.hlsl): what would end the pixel there gives no clouds instead.
#ifndef CLOUDS_MAIN
#define CLOUDS_MAIN main
#endif
#ifdef EDCAS_FINAL
#define DISCARD { o0 = 0; return; }
#else
#define DISCARD discard
#endif

void CLOUDS_MAIN(float3 v0 : TEXCOORD0, float3 v1 : TEXCOORD1, float3 v2 : TEXCOORD2, float3 v3 : TEXCOORD3,
          out float4 o0 : SV_Target0)
{
	float4 params = IniParams.Load(int2(503, 0));
	float4 params2 = IniParams.Load(int2(504, 0));
	float coverage = params.x, brightness = params.w;
	float opacity = params2.x, iridescence = params2.w;
	if (coverage <= 0 || opacity <= 0)
		DISCARD;

	// From space too: the layer lies over the planet's disc and fades out with distance (see
	// `away` below). Switched off outside the atmosphere, the clouds popped in on entry, after
	// the fade of their values had long finished in supercruise (2026-10-26).
	float3 cam = cb1[3].xyz;

	// The view ray and how far the ground (or anything else drawn) is along it, as the pass works
	// out the spot it lays haze on: nearer than the pass's own surface means something is there.
	// Sky pixels have no limit. A ship above the layer sees the clouds over the ground below it
	// (only sky pixels were drawn before, so low clouds under the ship showed as nothing but
	// their horizon edge, 2026-10-26). Read by pixel, not through a sampler: the pass's variants
	// bind their samplers to different slots (s1 in 236d6421, s2 in 79325565 and 8c9b2131).
	float3 dir;
	int2 pixel;
	float groundN = viewRay(v0, v1, dir, pixel);
	bool groundPixel = groundN >= 0;
	float ground = groundPixel ? groundN * cb1[31].w : 1e30;
#ifdef EDCAS_FINAL
	// The one cloud pass of the frame (edcas_clouds_final.hlsl), after Elite's last full-size
	// atmosphere draw: over the whole picture, with no stencil, band or flags to follow.
	float band = 1;
#else
	float band = bandWeight(v1);
	if (band <= 0)
		DISCARD;
	// Each of Elite's atmosphere draws does either the sky (cb1[18].x bit 1) or the haze over the
	// ground (bit 2), or both, and discards the other pixels; ground pixels are those nearer than
	// the pass's own surface. The clouds went on in every draw regardless, so twice where a sky
	// draw and a ground-only draw both cover the sky: a hard line where the ground draw's mesh
	// ends, 2.4 degrees under the horizon at 4.6 km on 6 e (2026-10-26). The same rule as the
	// pass, from its own code.
	int passFlags = asint(cb1[18].x);
	if (groundPixel ? (passFlags & 2) == 0 : (passFlags & 1) == 0)
		DISCARD;
#endif

	float3 pos = cam * cb1[31].w;
	float planet = cb1[31].z, top = cb1[31].w;
	Slab slab = loadSlab();
	float shell = slab.shellN * top;

	// ---- Thick mode: ray-march a slab around the shell (z505 > 0). Everything below this block is
	// the thin shell and runs unchanged when z505 = 0. ----
	float4 params5 = IniParams.Load(int2(505, 0));
	if (params5.z > 0) {
		// All lengths in atmosphere radii: the squared radii (1e13 in metres) lost the bottom
		// sphere's edge to rounding at float precision.
		float rPlanet = slab.rPlanet, shellN = slab.shellN, thickN = slab.thickN;
		float bottom = slab.bottom, slabTop = slab.top, span = slab.span;
		float k = slab.k, squeeze = slab.squeeze, threshold = slab.threshold;
		float3 p0 = cam;
		float cb = dot(p0, dir);
		float cc = dot(p0, p0);

		// The ray inside the top sphere (t >= 0), from below the slab, from inside it (starts at 0:
		// this shows flying through it as fog) or from above. A ray that dips into the bottom sphere
		// and out again goes on through the slab: the samples below it count as empty. Stopping at
		// the bottom sphere drew a hard line along its horizon, where a ray that just misses it
		// suddenly went on for many times as far (2026-10-26).
		float dTop = cb * cb - (cc - slabTop * slabTop);
		if (dTop <= 0)
			DISCARD;
		float sTop = sqrt(dTop);
		float t0 = max(-cb - sTop, 0), t1 = -cb + sTop;
		float dBot = cb * cb - (cc - bottom * bottom);
		if (dBot > 0 && cc < bottom * bottom)
			t0 = max(t0, -cb + sqrt(dBot));  // below the slab: begin where the ray leaves the bottom sphere
		// Only the drawn terrain hides the clouds, not a sphere at the planet's radius: the ground
		// can lie well below that radius, and cutting the rays at its horizon drew a hard line
		// across the sky, cloud above and none below, about 2 degrees under the horizon at 4.6 km
		// (2026-10-26). The horizon fade below hides the clouds further down.
		// Terrain hides what is behind it; a grazing ray stays bounded at 10 slab thicknesses.
		t1 = min(min(t1, groundN >= 0 ? groundN : 1e30), t0 + 10 * thickN);
		if (t1 <= t0)
			DISCARD;

		float reach5 = params5.y > 0 ? params5.y : 20;
		float away5 = 1 - smoothstep(reach5 / 3, reach5, (length(pos) - planet) / planet);
		if (away5 <= 0)
			DISCARD;

		// Outside the slab it fades towards the horizon like the shell does (a little wider, the
		// slab is seen through more haze); inside it there is nothing to fade. From above, by the
		// angle the ray meets the slab's top at: along the planet's edge seen from orbit the ray
		// grazes it through a long stretch of haze. By the camera's own elevation that edge was
		// in full view, and the clouds stood out against space (2026-10-26).
		float cameraR = sqrt(cc);
		float elevation5 = dot(dir, p0 / cameraR);
		float fade5 = 1;
		if (cameraR < bottom)
			fade5 = smoothstep(-0.05, 0.15, elevation5);
		else if (cameraR > slabTop)
			fade5 = smoothstep(0.02, 0.2, -dot(dir, normalize(p0 + dir * t0)));
		if (fade5 <= 0)
			DISCARD;

		float3 sun = -cb1[4].xyz;

		// x504 is the alpha of a vertical path through the whole slab at density 1.
		float sigma = -log(1 - min(opacity, 0.999)) / span;
		float3 sunColour = starLightTint() * cb1[8].xyz / max(max(cb1[8].x, cb1[8].y), max(cb1[8].z, 1e-30));

		// Two lobes (forward with a silver lining, a little backward), scaled so that the average
		// over all directions is 1 and capped at 4 so that looking into the sun does not blow out.
		float cosSun = dot(dir, sun);
		float g1 = 0.6, g2 = -0.2;
		float hg = 0.7 * (1 - g1 * g1) / pow(max(1 + g1 * g1 - 2 * g1 * cosSun, 1e-4), 1.5)
		         + 0.3 * (1 - g2 * g2) / pow(max(1 + g2 * g2 - 2 * g2 * cosSun, 1e-4), 1.5);
		float phase = min(hg, 4);

		// MARCH_STEPS at least, more along a long path, so that a step stays under half a cloud
		// feature (a noise unit of the base octave; k of them per atmosphere radius): the Thickness
		// setting lengthens the path (capped at ten slab thicknesses) while the features keep the
		// planet's size, and the fixed count skipped over them at 300 %; a tester saw grain painted
		// on the screen, growing with the setting (2026-11-08). Capped for the frame rate.
		float L = t1 - t0;
		int steps = clamp(int(ceil(L * k * 2)), MARCH_STEPS, MAX_STEPS);
		// Inside the slab (the fog-type clouds reach the ground at a high Thickness setting) the
		// nearest cloud is at the camera: the samples start short and lengthen (t grows as u^1.6),
		// so the first metres are not one coarse step with a pixel's worth of jitter in it.
		float power = cc < slabTop * slabTop && cc > bottom * bottom ? 1.6 : 1;
		// Detail finer than two steps along the ray cannot be sampled: as the ship moved, the steps
		// slid over it and the clouds flickered in moving waves (2026-10-26). So the octaves that
		// fine fade out, by the step length in noise units (the octaves halve in size).
		float lod = clamp(1 + log2(1 / max(2 * L / min(steps, 32) * k, 1e-6)), 1, 5);
		// An offset per screen pixel moves each ray's samples by up to one step, which turns the
		// banding of the steps into fine grain that the composite's tent averages out. (v0.xy are
		// clip coordinates, so the pixel the depth is read from is used.) The pattern is painted
		// on the screen (the offset is the pixel's), which is seen when the clouds move under it:
		// it is kept small (bayerJitter, the steps above), not moved (a per-frame offset would be
		// flicker, and the sky pass gives no last-frame camera for a temporal filter). Interleaved
		// gradient noise drew diagonal stripes in Elite's small sky buffer (2026-10-26); the Bayer
		// pattern showed none on ed_frame511315.
#ifdef EDCAS_FINAL
		// The half-size buffer: the pattern on its own pixels, smoothed by the composite.
		float jitter = bayerJitter(uint2(floor(float2(pixel.xy) * 0.5)));
#else
		float jitter = bayerJitter(uint2(pixel.xy));
#endif

		float3 col = 0;
		float T = 1;
		[loop] for (int i = 0; i < steps; i++) {
			float u0 = float(i) / steps, u1 = float(i + 1) / steps;
			float dtStep = L * (pow(u1, power) - pow(u0, power));
			float3 p = p0 + dir * (t0 + L * pow(lerp(u0, u1, jitter), power));
			float r = length(p);
			if (r < bottom)
				continue;  // inside the bottom sphere, under the slab
			float h = saturate((r - bottom) / span);
			// The warp at this sample: one plain noise shifts the pattern along a fixed slant (the
			// shell bends it with three). Worked out once per ray at its middle, it jumped wherever
			// the ray's end did, at every terrain edge and the slab's own, and cut the clouds along
			// straight lines there; three noises per step were a large part of the cost once the
			// clouds covered the whole picture at full size, 200 -> 50 FPS (2026-10-26).
			float3 atPlain = cloudCoord(p, k, shellN, squeeze, 0);
			float3 warp = (noise(atPlain * 0.5 + 3.7) - 0.5) * 1.6 * float3(1, -0.6, 0.8);
			float3 at = atPlain + warp;

			// Cheap skip: two octaves are far below the threshold's start, so five would be too.
			// Those two (fbm's first two noises, as `fbm(at, 2)` sums them) go on into the density.
			float lowCut = -0.25 * (1 - slabProfile(h));
			float n0 = noise(at), n1 = noise(at * 2.03 + 17.1);
			if ((0.5 * n0 + 0.25 * n1) / 0.75 + lowCut < threshold - 0.03 - 0.1)
				continue;
			float base = slabDensityLod(at, h, threshold, lod, n0, n1);
			if (base <= 0)
				continue;
			float erosionWeight = saturate(lod - 3);
			float erosion = 0;
			[branch] if (erosionWeight > 0)
				erosion = fbm(at * 4.1 + float3(5.2, 1.3, 8.7), 2) * erosionWeight;
			float density = saturate(base - (1 - base) * erosion * 0.5 * (1 + edges));
			if (density <= 0)
				continue;

			// The same day / horizon logic as the shell, at this sample's own up and radius.
			float day = dot(p / r, sun);
			float sky;
			float light = sunlight(day, rPlanet / r, sky);
			float3 tint = lerp(float3(1.0, 0.55, 0.35), float3(1, 1, 1), saturate(day * 3));

			// Sunlight left after the cloud between this sample and the sun: a few steps, longer
			// and longer, with a cheaper density.
			float tau = 0, travel = 0;
			// Three steps of up to 2 octaves (four of 3 before; the shadow is soft anyway, and it
			// cost), the second octave faded by what the tap's length resolves (slabDensityShadow).
			// Skipping them without sunlight (`light` 0) saved nothing measurable (PERFORMANCE.md C5).
			[unroll] for (int s = 0; s < 3; s++) {
				float len = (s == 0 ? 0.15 : s == 1 ? 0.3 : 0.55) * thickN;
				travel += len;
				float3 ps = p + sun * travel;
				float hs = (length(ps) - bottom) / span;
				if (hs > 0 && hs < 1) {
					float lodS = clamp(1 + log2(1 / max(2 * len * k, 1e-6)), 1, 2);
					tau += slabDensityShadow(cloudCoord(ps, k, shellN, squeeze, warp), hs, threshold, lodS) * len;
				}
			}
			// Powder: the inside of a thick cloud looks darker than its rim, the sun reaches less
			// of the scattering there; half strength.
			float sunlit = exp(-sigma * tau) * lerp(1, 1 - exp(-2 * sigma * density * thickN * 0.25), 0.5);
			// So that the undersides are not black: sky light, more at the top.
			float ambient = 0.25 * sky * (0.4 + 0.6 * h);
			float3 radiance = sunColour * (0.5 * brightness) * (sunlit * light * phase + ambient) * tint;

			float stepAlpha = 1 - exp(-sigma * density * dtStep);
			col += T * stepAlpha * radiance;
			T *= 1 - stepAlpha;
			if (T < 0.01)
				break;
		}

		float alpha = (1 - T) * fade5 * away5;
		if (alpha <= 0)
			DISCARD;
		// The blend is SRC_ALPHA INV_SRC_ALPHA, so the colour goes out straight, not premultiplied.
		float3 straight = col / max(1 - T, 1e-4);
		// Iridescence on the final colour, as on the shell (the hue follows the angle to the sun
		// and how dense the cloud is).
		float irid = iridescence * alpha * smoothstep(0.906, 0.985, cosSun);
		if (irid > 0) {
			float hue = acos(saturate(cosSun)) * 7 + (alpha - 0.5) * 1.2;
			float3 pastel = 1 + 0.35 * cos(6.2831853 * (hue + float3(0, 0.33, 0.67)));
			straight *= lerp(float3(1, 1, 1), pastel, irid);
		}
		o0 = float4(straight, inBand(alpha, band));
#ifdef EDCAS_FINAL
		o0.rgb *= o0.a;  // into the half-size buffer, premultiplied for its upscaling
#endif
		return;
	}

	// Where the view ray meets the cloud shell: the far side from below it, the near side from above.
	float b = dot(pos, dir);
	float c = dot(pos, pos) - shell * shell;
	float disc = b * b - c;
	if (disc < 0)
		DISCARD;
	float t = c < 0 ? -b + sqrt(disc) : -b - sqrt(disc);
	if (t <= 0 || t >= ground)
		DISCARD;
	float3 up = (pos + t * dir) / shell;  // unit vector at the hit point

	// Thin out towards the horizon, where the shell is seen at a grazing angle through haze:
	// looking up at it from below, or down on it from above.
	// From above by the angle at the hit point, as in the thick mode.
	float elevation = dot(dir, normalize(pos));
	float fade = c < 0 ? smoothstep(-0.02, 0.25, elevation) : smoothstep(0.02, 0.25, -dot(dir, up));
	if (fade <= 0)
		DISCARD;

	// Wind along one axis fixed to the planet: squeezing the noise coordinate along it stretches
	// the features into streaks where it lies flat (most of the sky), not near its two poles.
	// The axis must not depend on the camera: taken from the camera's position, the pattern slid
	// across the sky as fast as the ship moved over the planet (2026-10-26). (East at each hit
	// point stretched nothing: the coordinate there is the hit point's own up.)
	float3 east = float3(1, 0, 0);

	// Noise in layer heights, as the slab's (loadSlab).
	float3 at = up * (slab.shellN * slab.k);
	at += east * (dot(at, east) * (slab.squeeze - 1));

	// Domain warp: a low-frequency vector bends the streaks. One fbm and two plain noises.
	float3 warp = float3(fbm(at * 0.5 + 3.7, 3), noise(at * 0.5 + 31.7), noise(at * 0.5 + 57.3)) - 0.5;
	at += warp * 1.6;

	float f = fbm(at, 5);
	float base = smoothstep(slab.threshold - 0.03, slab.threshold + 0.12, f);
	if (base <= 0)
		DISCARD;

	// Erosion: fine noise bites into the thin parts the most, which leaves wispy edges. Half
	// strength: at full strength (and the narrower threshold above) the clouds averaged a sixth
	// of the density of the first version and vanished in game (2026-10-26).
	float erosion = fbm(at * 4.1 + float3(5.2, 1.3, 8.7), 3);
	float density = saturate(base - (1 - base) * erosion * 0.5);
	// Seen from far out the clouds would be finer than a pixel: they fade out up to y505 planet
	// radii above the ground (from a third of it on; 0 = the default 20). 1.5 radii hid them on
	// a supercruise approach until close; 20 looked much better in game (2026-10-26).
	float reach = IniParams.Load(int2(505, 0)).y;
	reach = reach > 0 ? reach : 20;
	float away = 1 - smoothstep(reach / 3, reach, (length(pos) - planet) / planet);
	float a = density * fade * away * saturate(opacity);
	if (a <= 0)
		DISCARD;

	// Sunlight at the hit point. The sun is still seen from there while it is not lower than the
	// horizon of the cloud shell, which lies below the horizon of the ground. So the clouds stay
	// lit a little after the ground goes dark (lit), and the part lit by day is as before.
	float3 sun = -cb1[4].xyz;
	float day = dot(up, sun);
	float sky;
	float light = sunlight(day, planet / shell, sky);
	float3 tint = lerp(float3(1.0, 0.55, 0.35), float3(1, 1, 1), saturate(day * 3));
	// A second sample a little towards the sun darkens the sides facing away from it.
	float shade = 1 - 0.6 * saturate(fbm(at + sun * 0.4, 5) - f + 0.1);

	// Iridescence: within about 25 degrees of the sun the tint shifts to a pastel palette whose
	// hue follows the angle to the sun (bands) and the cloud's own noise.
	float cosSun = dot(dir, sun);
	float irid = iridescence * a * smoothstep(0.906, 0.985, cosSun);
	if (irid > 0) {
		float hue = acos(saturate(cosSun)) * 7 + (f - 0.5) * 1.2 + erosion * 0.4;
		float3 pastel = 1 + 0.35 * cos(6.2831853 * (hue + float3(0, 0.33, 0.67)));
		tint = lerp(tint, pastel, irid);
	}

	// Sunlit ice in the sun's colour (cb1[8], normalised). The pass's target is already exposed,
	// 1 being white, so brightness 1 is ice at half of white in full sun. cb1[8] x exposure itself
	// came to 1e-4..1e-2, too small for the clouds' own scale, 2026-10-26.
	float3 sunColour = starLightTint() * cb1[8].xyz / max(max(cb1[8].x, cb1[8].y), max(cb1[8].z, 1e-30));
	float3 radiance = sunColour * (0.5 * brightness) * light * shade * tint;
	o0 = float4(radiance, inBand(a, band));
#ifdef EDCAS_FINAL
	o0.rgb *= o0.a;  // into the half-size buffer, premultiplied for its upscaling
#endif
}
