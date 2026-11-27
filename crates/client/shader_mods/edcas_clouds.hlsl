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
//   x501, y501 the layer's base over the ground (the radius w507) and the slab's thickness, metres
//        (EDCAS works them out in scale heights; 0 = placed by z503 and z505 in the shell as before)
//   z501 the wind the clouds drift with, m/s (0 = still, as before 0.43.124); z515 is the clock
//   x502 how strongly the air reddens a low sun, by its column mass (0 = 1, Earth's (1, 0.55, 0.35)):
//        only without row 550
//   x550 y550 z550 the air's Rayleigh optical depth straight up from the ground in red, green and blue
//        (airTint: the sunlight's filter by the height of the point and the sun over it; 0 = lowSunTint)
//   w550 the methane over the ground, km-amagat (methaneTint, its absorption in the red; 0 = none)
//   y502 the air's scale height in metres (0 = unknown: every layer darkens after sunset as a low one)
//   w501 how strongly the cover follows the local time (0 = not at all, as before 0.43.125; 1 = 1.43
//        times x503 at the terminator and on the night side, 0.57 times under the sun)
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

// The sky pass's constants and its depth buffer are where that pass has them. A pass of ours that runs
// on another of Elite's draws, whose own textures and constants must stay as they are, names slots of
// its own before it includes this file (edcas_cloud_shade.hlsl: the sun pass).
#ifndef EDCAS_CB1_SLOT
#define EDCAS_CB1_SLOT b1
#endif
#ifndef EDCAS_DEPTH_SLOT
#define EDCAS_DEPTH_SLOT t0
#endif

cbuffer cb1 : register(EDCAS_CB1_SLOT)
{
	float4 cb1[334];
}


Texture2D<float4> Depth : register(EDCAS_DEPTH_SLOT);
Texture2D<float4> DepthAlt : register(t4);
Texture1D<float4> IniParams : register(t120);
// Elite's own shadow of the sun, a copy of the sun pass's t3 (edcas.ini [CommandListEdcasSunset]): 1 where
// the sun reaches what a pixel shows, 0 where Elite's shadow maps keep it off, the edges soft between.
// The sun pass multiplies the sun's colour by it and by nothing else (README-ELITE.md, "Constants of the
// sun pass"). For the ground probe (edcas_ground_grid.hlsl), and through it the fog's light (edcas_fog.hlsl).
Texture2D<float> SunShade : register(t113);

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

// The clouds' motion (2026-11-11, client 0.43.124; CLOUDS-RESEARCH.md §5.5: high clouds drift at 15 to
// 107 m/s, clouds form and dissolve within hours). Set by loadSlab from the wind z501 and the clock z515
// (seconds since the game started, as the dust has it). drift: how far the wind has carried the pattern,
// in the base octave's noise units, along the planet frame's fixed x axis, the axis the streaks are
// squeezed along and the dust blows along (edcas_dust.hlsl: the same direction). evolve: the clock in
// units of the clouds' change, run through the warp (shapes change over about EVOLVE_S) and the
// erosion (wisps, four times as fast), along each point's own up, which changes the pattern in place.
static float3 drift = 0;
static float evolve = 0;
#define EVOLVE_S 1200.0

// Value noise at a lattice cell i (whole numbers) and the place f in it (0..1), on the float hash of
// edcas_noise.hlsl: noise(x) is noiseCellF(floor(x), frac(x)).
float noiseCellF(float3 i, float3 f)
{
	f = f * f * (3.0 - 2.0 * f);
	return lerp(lerp(lerp(hash(i + float3(0, 0, 0)), hash(i + float3(1, 0, 0)), f.x),
	                 lerp(hash(i + float3(0, 1, 0)), hash(i + float3(1, 1, 0)), f.x), f.y),
	            lerp(lerp(hash(i + float3(0, 0, 1)), hash(i + float3(1, 0, 1)), f.x),
	                 lerp(hash(i + float3(0, 1, 1)), hash(i + float3(1, 1, 1)), f.x), f.y), f.z);
}

// noise(x) of the drifting pattern, where x is the base coordinate times `scale` (plus an offset): the
// drift goes onto the place in the cell, not onto x, as fbmMoving does in the dust (README-ELITE.md,
// "Float precision"): x is tens to a thousand noise units, where a float holds the drift of a frame
// (a millionth of a unit for a deck at 7 m/s) only to a few of its steps. With drift 0 it is noise(x).
float noiseD(float3 x, float scale)
{
	float3 i = floor(x);
	float3 f = (x - i) - drift * scale;
	float3 whole = floor(f);
	return noiseCellF(i + whole, f - whole);
}

// fbm of the drifting pattern, n octaves, the first at `scale` times the base coordinate.
float fbmD(float3 p, const int n, float scale)
{
	float sum = 0, amp = 0.5, total = 0;
	[unroll] for (int i = 0; i < n; i++) {
		sum += amp * noiseD(p, scale);
		total += amp;
		p = p * 2.03 + 17.1;
		scale *= 2.03;
		amp *= 0.5;
	}
	return sum / total;
}

// The spread (standard deviation) of one octave of value noise, noiseD: for the variance fbmFoot takes away.
#define NOISE_SD 0.17

// fbmD filtered to a footprint (`foot`, in units of p; the first octave's cells are 1 unit): an octave whose
// cells are under two footprints gives its mean, 0.5, as the waves are filtered (waveKeep), and `lost` comes back
// with the spread that took away, in fbm's units, for a threshold's edge to widen by: the cover's mean then stays
// where the pixel holds many cells (2026-11-26: the high clouds' shell from orbit, ed_frame222039, a cell some
// tenths of a pixel, one fbmD read a pixel thresholded into glitter over the whole disc).
float fbmFoot(float3 p, const int n, float scale, float foot, out float lost)
{
	float sum = 0, amp = 0.5, total = 0, cell = 1;
	lost = 0;
	[unroll] for (int i = 0; i < n; i++) {
		float keep = 1 - smoothstep(0.25 * cell, 0.5 * cell, foot);
		float v = keep > 0 ? noiseD(p, scale) : 0.5;
		sum += amp * lerp(0.5, v, keep);
		lost += amp * amp * (1 - keep * keep);
		total += amp;
		p = p * 2.03 + 17.1;
		scale *= 2.03;
		amp *= 0.5;
		cell /= 2.03;
	}
	lost = sqrt(lost) / total * NOISE_SD;
	return sum / total;
}

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
	float sum = 0, amp = 0.5, total = 0, scale = 1;
	[unroll] for (int i = 0; i < 5; i++) {
		float w = saturate(octaves - i);
		float n = i == 0 ? n0 : n1;
		if (i > 1) {
			n = 0;
			[branch] if (w > 0)
				n = noiseD(p, scale);
		}
		sum += amp * w * n;
		total += amp * w;
		p = p * 2.03 + 17.1;
		scale *= 2.03;
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
//
// Since client 0.43.126 the power falls with the point's height over the ground in the air's scale
// heights (y502), 1 + (w506 - 1) exp(-h / H): the sun's last light reaches a point at h along a ray
// that grazes the air no lower than about h - R d^2 / 2 (d the sun's depression), so a high layer is
// lit through thin air almost to the end of its own window, d = sqrt(2 h / R) (CLOUDS-RESEARCH.md §5.3,
// §6.4 anchor 3: high clouds lit against a darkening sky are the most spectacular thin-air clouds).
// A low deck (6 e's, 0.2 to 0.4 H) keeps 3 to 3.5, the main gas's high clouds (5 H) 1.
float twilightPower(float heightM)
{
	float power = IniParams.Load(int2(506, 0)).w;
	power = power > 0 ? power : 4;
	float scaleHeight = IniParams.Load(int2(502, 0)).y;
	return scaleHeight > 0 ? 1 + (power - 1) * exp(-max(heightM, 0) / scaleHeight) : power;
}

float twilight(float day, float limit, float power)
{
	return pow(smoothstep(-limit, 0.02, day), power);
}

// The colour of the sunlight under a low sun at day: towards (1, 0.55, 0.35) (the clouds; the fog's own
// `red`) as the sun nears the horizon, as through Earth's air, scaled by x502: the air's column mass
// over a dense one's (EDCAS: p / g over 10 000 kg/m², Earth's column; thin air reddens little, §5.2;
// 0 = 1, as before 0.43.126).
float3 lowSunTint(float day, float3 red)
{
	float strength = IniParams.Load(int2(502, 0)).x;
	strength = strength > 0 ? saturate(strength) : 1;
	return lerp(lerp(float3(1, 1, 1), red, strength), float3(1, 1, 1), saturate(day * 3));
}

// ---- The sunlight through the air (client 0.43.224) ----
//
// What the air leaves of the star's light on its way to a point, per colour: exp(-tau), tau the air's
// Rayleigh optical depth along the way to the sun. It replaces lowSunTint wherever EDCAS has the numbers
// (row 550, the scale height y502, the radius w507): that one blended towards a fixed orange under 19
// degrees, whatever the height of the point.
//
// Row 550 x y z: the optical depth of the whole air straight up from the ground, in red (610 nm), green
// (550 nm) and blue (465 nm), from EDCAS: Earth's (0.0637, 0.0971, 0.193; Bodhaine et al. 1999, "On
// Rayleigh optical depth calculations", table 3) times the planet's count of molecules over a square
// metre against Earth's and each gas's scattering against air's (planet_clouds.rs `rayleigh`). At a
// height h there is exp(-h / H) of it overhead (H the scale height y502).
//
// The slant: chapmanColumn, the air along the ray to the sun over the air straight up from the ground
// (Chapman's function times exp(-h / H)), as the haze has it: 1 at the zenith from the ground,
// sqrt(pi x / 2) at the horizon (x the point's radius in scale heights: 26 on a 3 578 km planet with
// H = 8 km, 34 on Earth), within 2 % of the integral. With the sun under the point's horizontal, as a
// high cloud sees it after the ground's sunset, the ray dips towards the ground and comes back up:
// twice the horizon's air of its lowest point less the stretch behind the point, which is exact (Ch(x,
// chi) = 2 exp(x - x sin chi) Ch(x sin chi, 90) - Ch(x, 180 - chi)). That is the red of the last light on
// a high cloud, by the air's own numbers. The planet's own shadow is sunlightAt's.

// exp(z^2) erfc(z) for z >= 0: a rational fit with the function's value, slope and curvature at 0
// (1, -2 / sqrt(pi), 2) and its fall as 1 / (sqrt(pi) z) far out, within 2.5 % between. The slope and
// the curvature at 0 are what make the column run smoothly through a level ray and through a ray that
// only just touches the haze's ground level.
float erfcx(float z)
{
	return (1 + 0.42424 * z) / (1 + 1.55262 * z + 0.75195 * z * z);
}

// The column from a point at height h (scale heights) over a ground of radius X (scale heights) to
// infinity along a ray at the cosine c to the point's zenith, in vertical columns from the ground.
float chapmanColumn(float X, float h, float c)
{
	float x = X + h;
	if (c >= 0)
		return sqrt(1.5707963 * x) * erfcx(c * sqrt(0.5 * x)) * exp(-h);
	// Falling: twice the column from the ray's lowest point (level there), less the column upwards
	// behind the point.
	float x0 = sqrt(max(1 - c * c, 0)) * x;
	// (Held under a float's largest: far under the horizon the difference is past any light anyway.)
	return 2 * sqrt(1.5707963 * x0) * exp(min(X - x0, 80)) - sqrt(1.5707963 * x) * erfcx(-c * sqrt(0.5 * x)) * exp(-h);
}

// Methane's absorption (client 0.43.225): its bands at 543, 576, 619, 666 and 727 nm take red first, then
// green; blue stays (Karkoschka 1998, Icarus 133, 134: the coefficients of PDS ESO-J/S/N/U-SPECTROPHOTOMETER-4-V2.0,
// 1995low.tab, 1 nm resolution, the giant planets' temperature). A colour channel's share of a sun-like star's
// light (5778 K, through the CIE 1931 observer to linear sRGB) left after a column of u km-amagat is not
// exp(-k u) for any one k: the narrow bands saturate while the light between them passes. So it is a sum of
// exponentials fitted to the spectrum's own sum over u from 0.01 to 1000 km-am, within 0.4 % in red, 0.13 % in
// green and 1.5 % in blue (`ed-captures/cloud-shadow/methane_fit.py`); the last weight never goes. 3000 K and
// 9000 K stars differ from it by under 3 % at 10 km-am. Titan's column, 2.86 km-am (Tokano et al. 2006),
// leaves (0.80, 0.96, 1) of the light straight down and (0.21, 0.54, 0.91) along its horizon.
float3 methaneTint(float u)
{
	float r = 0.29670 * exp(-0.00883 * u) + 0.46629 * exp(-0.03761 * u) + 0.21383 * exp(-0.40863 * u) + 0.02318;
	float g = 0.49346 * exp(-0.00273 * u) + 0.32284 * exp(-0.01307 * u) + 0.16721 * exp(-0.06431 * u) + 0.01649;
	float b = 0.55061 * exp(-0.00129 * u) + 0.44939;
	return float3(r, g, b);
}

// The air's numbers for airTintOf, read once (a march reads them before its loop).
struct AirColumn {
	float3 depth;      // row 550: the vertical optical depth at the ground, red, green, blue
	float methane;     // w550: the methane over the ground, km-amagat
	float scaleHeight; // y502, metres
	float radiusM;     // w507
};

AirColumn loadAirColumn()
{
	AirColumn a;
	float4 row = IniParams.Load(int2(550, 0));
	a.depth = row.xyz;
	a.methane = max(row.w, 0);
	a.scaleHeight = IniParams.Load(int2(502, 0)).y;
	a.radiusM = IniParams.Load(int2(507, 0)).w;
	return a;
}

// The sunlight's colour and strength at a point heightM over the ground's radius where the sun stands
// at `day` (the sine of its height there): the air's own filter, or, without the numbers (a client
// before 0.43.224, an unknown gravity), lowSunTint towards `red` as before.
float3 airTintOf(AirColumn a, float day, float heightM, float3 red)
{
	float3 tint = lowSunTint(day, red);
	if (a.depth.x + a.depth.y + a.depth.z + a.methane > 0 && a.scaleHeight > 0 && a.radiusM > 0) {
		// The air on the way to the sun over the air straight up from the ground: the same for every gas
		// mixed through it, so the methane's column goes the same way as the scattering's.
		float column = max(chapmanColumn(a.radiusM / a.scaleHeight, max(heightM, 0) / a.scaleHeight, day), 0);
		tint = exp(-a.depth * column);
		if (a.methane > 0)
			tint *= methaneTint(a.methane * column);
	}
	return tint;
}

float3 airTint(float day, float heightM, float3 red)
{
	return airTintOf(loadAirColumn(), day, heightM, red);
}

// The sunlight at a point with the sun at day (the sine of its height there) and the planet's
// radius over the point's (both from its centre): the sun is seen while it is not lower than
// the point's own horizon, which lies below the ground's, so high clouds stay lit a little after
// the ground goes dark. sky: the share of the sky light left (dusk); the return value adds the
// sun's own height. Shared by the clouds, the fog and the canopy frost.
float sunlightAt(float day, float planetOverR, float power, out float sky)
{
	float limit = sqrt(max(1 - planetOverR * planetOverR, 0));
	sky = smoothstep(-limit - 0.03, -limit + 0.03, day) * twilight(day, limit, power);
	return sky * max(saturate(day * 4 + 0.15), 0.35);
}

// As sunlightAt with the curve's power at the ground (w506).
float sunlight(float day, float planetOverR, out float sky)
{
	return sunlightAt(day, planetOverR, twilightPower(0), sky);
}

// Iridescence only at dawn and dusk (since 0.43.126): fresh, uniform crystals of high clouds lit
// by a low sun (CLOUDS-RESEARCH.md §5.3, Mars's twilight clouds): full with the sun under 6 degrees at
// the cloud, gone from 17 degrees.
float dawnOnly(float day)
{
	return 1 - smoothstep(0.1, 0.3, day);
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
	float f = fbmD(at, octaves, 1) - 0.25 * (1 - slabProfile(h));
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
		second = noiseD(at * 2.03 + 17.1, 2.03);
	float f = (0.5 * noiseD(at, 1) + 0.25 * w * second) / (0.5 + 0.25 * w) - 0.25 * (1 - slabProfile(h));
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

// The warp of the slab's noise at a point (its coordinate atPlain, its up): one plain noise shifts the
// pattern along a fixed slant, drifting with the pattern and changing with the clock (evolve).
float3 slabWarp(float3 atPlain, float3 up)
{
	return (noiseD(atPlain * 0.5 + 3.7 + up * evolve, 0.5) - 0.5) * 1.6 * float3(1, -0.6, 0.8);
}

// The slab's geometry in atmosphere radii and its noise scale, from the IniParams (z503, y503, x503,
// y504, z505 and the ceiling y506); also sets the look the slab shares (z506, x508, y508 into tops, edges,
// bases). For main's thick mode and the canopy's cloudAt (edcas_canopy_contact.hlsl).
struct Slab {
	float rPlanet, shellN, thickN, bottom, top, span, k, squeeze, threshold, coverage, localTime, groundN, wave, driftU, cells, rows, tail, smooth, halo, droplets;
};

// ---- Waves in the high layers (since 0.43.131; CLOUD-TYPES-PLAN.md phase 2) ----
//
// Undulatus, and the noctilucent forms' bands (type 2) and billows (type 3): crests across the wind,
// whose axis is the planet frame's x, the one the layers drift along and their streaks are squeezed
// along. Two scales in the noise's unit (half the layer's height over the atmosphere's base at the
// default size y503 8): bands a unit apart (20 to 25 km for a layer at 40 km; Earth's noctilucent
// bands are tens of km apart, Mars's wave clouds 30 to 40 km, CLOUDS-RESEARCH.md S17), billows a fifth
// of that riding on the bands' crests only. The wave lifts the noise the cover is thresholded on, by
// WAVE_LIFT times the amplitude EDCAS writes (x529 the clouds' own layer, y529 the second layer; 0 =
// none): 0.2 at 2 scale heights to 0.6 from 6 up, waves grow as the density falls (§2.2), the thin
// veil half that. Its mean over a period is 0; the threshold rises by a keep times the amplitude
// squared times the threshold's distance into the noise's tail (threshold - 0.5), so the cover's mean
// over the sky stays (a lift helps the crests more than it costs the troughs, the cover being a tail of
// the noise): 0.5 in the slab, 0.04 in the thin shells, whose slant-path alpha nearly saturates; tuned
// on replays, the clouds' mean light within 4 % (ed_frame171605 at its cover and five times it, the
// second layer on ed_frame511315). The crests bend with the warp, and drift with the
// layer: the phase is split into the place in the wavelength and the drift's, as noiseD splits its
// cells (README-ELITE.md, "Float precision"). Filtered as the noise is: a scale whose wavelength is
// under two footprints (the march's step, the pixel) gives its mean, 0.
#define WAVE_BAND 1.0
#define WAVE_BILLOW 0.2
#define WAVE_LIFT 0.12
#define WAVE_KEEP_SLAB 0.5
#define WAVE_KEEP_SHELL 0.04

// How much of a wave of this wavelength a footprint (noise units) keeps.
float waveKeep(float lambda, float foot)
{
	return 1 - smoothstep(0.25 * lambda, 0.5 * lambda, foot);
}

// The noise's lift by the waves at the unsqueezed coordinate xU along the wind (noise units), the
// layer's drift dU along it, a bend (noise units), a footprint and the threshold it lowers.
float waveLift(float amp, float xU, float dU, float bend, float foot, float threshold, float keep)
{
	float pb = frac(xU / WAVE_BAND) - frac(dU / WAVE_BAND) + bend / WAVE_BAND;
	float ps = frac(xU / WAVE_BILLOW) - frac(dU / WAVE_BILLOW) + bend / WAVE_BILLOW;
	float band = cos(6.2831853 * pb);
	float w = 0.7 * band * waveKeep(WAVE_BAND, foot) + 0.45 * max(band, 0) * cos(6.2831853 * ps) * waveKeep(WAVE_BILLOW, foot);
	return amp * (WAVE_LIFT * w - keep * amp * max(threshold - 0.5, 0));
}

// ---- The buffered deck by night and by day (since 0.43.132; CLOUD-TYPES-PLAN.md phase 3) ----
//
// A low deck is cooled from its top at night, and the overturning that drives makes closed cells with
// clear gaps (stratocumulus stratiformis, perlucidus, lacunosus; CLOUD-TYPES-THIN-ATMOSPHERES.md §1, §3,
// §4; Mars's night clouds, S15). By day the ground heats the air under the deck, and in a steady wind
// the convection lines up into rows along it (radiatus, cloud streets; §4, Mars around Tharsis). EDCAS
// writes the two strengths (z529 the cells, w529 the rows, 0 = none; the rows only where the deck's wind
// is 6 m/s or more); the shader blends them by the sun's height at the cloud (the value the cover's
// local time uses): cells from 3 degrees under the horizon, full from 14 under; rows from 6 degrees up,
// full from 20 up; neither at the terminator, where the deck is as before.
//
// Cells: one value noise at 2.5 slab thicknesses, its mid-level contour as the gaps (the distance to
// the noise's mid-level, under DECK_GAP, is clear): closed cells with thin gaps from the noise there
// is, no cellular noise. Rows: cos(N phi) about the wind's axis (the planet frame's x), N whole so the
// rows close round it, about 3 slab thicknesses apart where the wind lies flat; they converge towards
// the axis's two poles as the streaks do, and are filtered by their spacing there (0.43.140: by the
// spacing at the axis's equator they stood as thin spokes over the horizon near a pole). Both bent by the warp, the cells drifting with the deck; both
// lift the noise the cover is thresholded on around a mean of 0 (the cells' mask less its mean DECK_CELL_MEAN),
// and the threshold rises by a keep (DECK_KEEP_CELLS, DECK_KEEP_ROWS) times the strength squared times its
// distance into the noise's tail, so the deck's mean cover stays (tuned on replays, as the waves'). Filtered to the ray's
// footprint: a cell or row under two footprints gives its mean.
#define DECK_CELL 2.5
#define DECK_ROW 3.0
#define DECK_GAP 0.1
#define DECK_CELL_MEAN 0.8
#define DECK_LIFT 0.2
#define DECK_KEEP_CELLS 1.55
#define DECK_KEEP_ROWS 1.85

// The value noise at x (x = coordinate * scale) shifted by `shift` along x, split as noiseD splits its
// cells (the drift onto the place in the cell).
float noiseShift(float3 x, float shift)
{
	float3 i = floor(x);
	float3 f = (x - i) - float3(shift, 0, 0);
	float3 whole = floor(f);
	return noiseCellF(i + whole, f - whole);
}

// The deck's lift of the noise at a point (its up, the warp there, the sun's height day), filtered to a
// footprint in noise units, and the threshold it lowers.
float deckLift(Slab s, float3 up, float3 warp, float day, float foot, float threshold)
{
	float thickU = max(s.thickN * s.k, 1e-4);
	float night = s.cells * smoothstep(-0.05, -0.25, day);
	float sunny = s.rows * smoothstep(0.1, 0.35, day);
	float w = 0;
	float keep = 0;
	[branch] if (night > 0) {
		float size = DECK_CELL * thickU;
		float3 c = up * (s.shellN * s.k) / size + warp * 0.25;
		float m = smoothstep(0, DECK_GAP, abs(noiseShift(c, s.driftU / size) - 0.5));
		float a = night * waveKeep(size, foot);
		w += a * (m - DECK_CELL_MEAN) / (1 - DECK_CELL_MEAN) * 0.5;
		keep += DECK_KEEP_CELLS * a * a;
	}
	[branch] if (sunny > 0) {
		float spacing = DECK_ROW * thickU;
		float n = max(round(6.2831853 * s.shellN * s.k / spacing), 1);
		// The rows' spacing on the sky shrinks with the distance from the wind's axis, to nothing at its
		// two poles: filter by the spacing here, so they go to their mean where they converge.
		float spacingHere = spacing * max(length(up.yz), 1e-4);
		float phi = atan2(up.z, up.y);
		float rows = cos(n * phi + warp.y * 3.27);
		float a = sunny * waveKeep(spacingHere, foot);
		w += a * rows;
		keep += DECK_KEEP_ROWS * a * a;
	}
	return DECK_LIFT * w - keep * max(threshold - 0.5, 0);
}

// The noise threshold for the cover x503 (from the noise's spread, mean 0.5, sd 0.11 after the warp,
// so that about x503 of the sky is cloud: before, coverage 0.25 (a sparse planet from EDCAS) left 1 %.
// In an offline port of the slab (camera on the ground, z503 0.24, z505 0.3) x503 = 0.25 covered 23 %
// of the sky above 10 degrees, 2026-10-26), at a point where the sun stands at `day` (the sine of its
// height). The cover follows the local time (2026-11-11, client 0.43.125; CLOUDS-RESEARCH.md §2.5,
// §5.5, §6.4 anchor 5: thickest at dawn and dusk, thinnest at noon, ice clouds at night): times
// (1.5 - 0.9 day) / 1.05 by day, 1.43 at the terminator to 0.57 under the sun, whose mean over the
// day side (day spread evenly over 0..1 by area) is 1, and the terminator's 1.43 on the night side,
// all scaled by w501 (s.localTime; the main gas's high clouds get 1.5 from EDCAS: they belong to dawn,
// dusk and night). On a slow or locked body this alone puts the clouds at the terminator (§2.7).
float slabThreshold(Slab s, float day)
{
	float local = 1 + s.localTime * ((1.5 - 0.9 * saturate(day)) / 1.05 - 1);
	return 0.5 + 0.3 * (0.5 - s.coverage * max(local, 0));
}

// The same with a share of cover added where the waves lift the noise (waveLift / 0.3): written so that
// with none the threshold is the one above to the bit, as the compiler folds it.
float slabThresholdWaved(Slab s, float day, float cover)
{
	float local = 1 + s.localTime * ((1.5 - 0.9 * saturate(day)) / 1.05 - 1);
	return 0.5 + 0.3 * (0.5 - (s.coverage * max(local, 0) + cover));
}


// ---- Fall streaks and hooks under ice veils (since 0.43.134; CLOUD-TYPES-PLAN.md phase 4) ----
//
// Ice crystals fall out of a veil's denser heads and sublimate on the way down: virga, the fibrous
// tails of cirrus, hooked (uncinus) where the tail falls into slower air and trails behind its head
// (CLOUD-TYPES-THIN-ATMOSPHERES.md §3, §5; Phoenix saw such snow streaks under Mars's night clouds,
// S14). EDCAS writes their length in slab thicknesses (x530, 0 = none; ice veils only): longer the
// thinner the air (crystals fall faster), shorter the further the gas is above its frost point at the
// ground (they sublimate sooner). In the slab march the samples under the slab's base, down to that
// length, look up the head above them: the cloud at a third of the slab's height, VIRGA_SLANT times the
// depth further downwind (the head moves on with the faster wind aloft, so the tail lags behind it),
// thinned with the square of the depth's share of the length, in strands (a finer noise of the head's
// spot, the same down the fall line), with their own few samples along the ray's stretch under the base
// (since 0.43.142; the slab's march is the one with no tails). The tails are lit as the slab's base (the
// head above shades them). The head is looked up where the slab's erosion leaves cloud, so the tails add little mass: the
// clouds' mean light with tails 1.2 slab thicknesses long is 1.04 to 1.05 of before on ed_frame171605
// (a thin slab at 40 km, at its cover and five times it), within the plan's 10 % with no compensation;
// the layer's top stays where it was. Off by default (EDCAS writes 0). With the slab's march stretched
// down over them (0.43.134) tails under the whole deck of ed_frame511315 cost +0.44 to +0.54 ms of GPU
// time, over the plan's 0.15 ms. Their own samples cut the noise evaluations they add there to a third
// (+3.0 against +9.4 on a count of them per pixel, +0.6 on ed_frame171605's veil): about +0.15 ms, not
// measured (the machine's timing was +- 0.5 ms and more on 2026-11-12), so they stay off.
// The thin shell (z505 = 0) has no depth to hang tails in and is left as it is.
#define VIRGA_SLANT 1.5
#define VIRGA_DENSITY 0.5
#define VIRGA_SAMPLES 4
#define VIRGA_MAX_SAMPLES 16

// The density of a tail at the point p (atmosphere radii) under the slab, its depth `below` the base
// (atmosphere radii) and the tail's length; the head's density as a by-product, for the tail's shade.
float virgaDensity(Slab s, float3 p, float below, float length_, float3 sun, out float head)
{
	float share = below / length_;
	float3 ph = normalize(p + float3(VIRGA_SLANT * below, 0, 0)) * (s.bottom + s.span / 3);
	float3 atPlain = cloudCoord(ph, s.k, s.shellN, s.squeeze, 0);
	float3 up = normalize(ph);
	// The head's cloud: three octaves over the threshold, a linear ramp of 0.25 from 0.1 over it, where
	// the slab's erosion leaves cloud (slabDensity's own ramp here changed the compiled march's rounding
	// with no tails).
	float threshold = slabThreshold(s, dot(up, sun));
	float3 at = atPlain + slabWarp(atPlain, up);
	head = 0;
	// Most tail samples lie under clear sky: the base octave alone tells, exactly, where the three
	// cannot reach the ramp ((0.5 n0 + 0.375) / 0.875 at most), and the rest is not worked out.
	float n0 = noiseD(at, 1);
	if (n0 < 1.75 * (threshold + 0.1) - 0.75)
		return 0;
	head = saturate(((0.5 * n0 + 0.25 * noiseD(at * 2.03 + 17.1, 2.03) + 0.125 * noiseD(at * 4.1209 + 51.813, 4.1209)) / 0.875 - threshold - 0.1) * 4);
	if (head <= 0)
		return 0;
	// Fibres: a finer noise of the head's spot, the same all the way down the fall line, so the tail
	// falls in strands (fibratus) rather than as a haze.
	float fibre = saturate(noiseD(at * 4.1 + 23.7, 4.1) * 2.5 - 1);
	float fade = 1 - share;
	return head * fibre * VIRGA_DENSITY * fade * fade;
}

// ---- The smooth veil and its 22 degree halo (since 0.43.135; CLOUD-TYPES-PLAN.md phase 5) ----
//
// A broad veil of water ice in light wind is a featureless sheet, cirrostratus nebulosus
// (CLOUD-TYPES-THIN-ATMOSPHERES.md §2, §3), and its hexagonal crystals refract the sun into the 22 degree
// halo (CLOUDS-RESEARCH.md §5, S41: the 60 degree prism's minimum deviation; CO2 ice is cubic, its halos
// only modelled, so none). EDCAS writes how smooth the veil is (y530, 0 = as before, 1 = smooth) and the
// halo's strength (z530, 0 = none; water ice only). Smoothness takes the streaks (the squeeze along
// the wind) and SMOOTH_EROSION of the erosion's wisps away, and spreads the veil over 1 + SMOOTH_COVER
// times the cover with its extinction divided by SMOOTH_THIN: a wide thin sheet with the veil's mean
// opacity (tuned on replays). The halo
// is a narrow rise of the sunlight's phase function, at most HALO_PEAK over the veil's own brightness,
// sharp on its red inner edge and fading outwards through white (no light scatters inside 22 degrees),
// seen through the veil from below it with the sun up.
#define SMOOTH_EROSION 0.8
// Droplets against ice (since 0.43.136; CLOUD-TYPES-PLAN.md phase 6): a layer of droplets (EDCAS's w530,
// the gas condensing above its triple point; CLOUD-TYPES-THIN-ATMOSPHERES.md summary 3) has no virga,
// halo or iridescence (EDCAS writes those 0) and softer edges: DROPLET_EROSION of the erosion's wisps
// taken away, droplets do not fall out in streaks, and the density thinned by DROPLET_KEEP where the
// erosion acts so the layer's mean light stays (tuned on replays).
#define DROPLET_EROSION 0.5
#define DROPLET_KEEP 0.7
#define SMOOTH_COVER 1.0
#define SMOOTH_THIN 6.0
#define HALO_DEG 22.0
#define HALO_PEAK 0.15

// The halo's tint of the sunlight at the cosine cosSun between the view and the sun, strength `halo`,
// the sun's height at the camera `sunUp` (its sine): none with the sun under the camera's horizon
// (0.43.143: the ring showed faintly at twilight through the sky-lit share).
float3 haloTint(float cosSun, float halo, float sunUp)
{
	halo *= smoothstep(-0.01, 0.02, sunUp);
	float deg = degrees(acos(clamp(cosSun, -1, 1)));
	// Inside 22 degrees nothing (a sharp inner edge over half a degree), outside a fall over 3 degrees.
	float inner = smoothstep(HALO_DEG - 0.5, HALO_DEG, deg);
	float ring = inner * exp(-max(deg - HALO_DEG, 0) / 3.0);
	// Red at the inner edge, white further out.
	float3 colour = lerp(float3(1.0, 0.55, 0.35), float3(1, 1, 1), saturate((deg - HALO_DEG) / 2.0));
	return 1 + HALO_PEAK * halo * ring * colour;
}

// The cover a smooth veil (y530) adds where the sun stands at `day`: it spreads into a wide, thin sheet
// over 1 + SMOOTH_COVER times its cover (the extinction divided by SMOOTH_THIN to the same power).
float smoothSpread(Slab s, float day)
{
	float local = 1 + s.localTime * ((1.5 - 0.9 * saturate(day)) / 1.05 - 1);
	return s.coverage * max(local, 0) * SMOOTH_COVER * s.smooth;
}

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
	// In metres over the ground (2026-11-11, client 0.43.123): x501 the base over the planet's radius
	// (w507, the journal's, which is the ground shaders' radius cb2[12].w to the metre), y501 the
	// thickness, both in scale heights worked out by EDCAS. cb1[31].z, the reference above, is the
	// atmosphere's base, 7 to 17 km under the ground on the bodies measured, and the shell 102 to 125 km
	// deep whatever the air's scale height, so shares of it put a "low" deck 15 km under the ground
	// on one body and 9 km over it on another (RenderDoc, renderdoc/heightprobe.py). Metres per
	// atmosphere radius are cb1[31].w * 1000, as the dust has them. The slab stays inside the drawn
	// atmosphere (under its top, where the ray march ends) and is never thicker than two thirds of its
	// base's height over cb1[31].z: the noise's features are half the layer's height over that wide,
	// so a slab thicker than that would stand taller than wide (thin air has no towers).
	float4 params1 = IniParams.Load(int2(501, 0));
	float radiusM = IniParams.Load(int2(507, 0)).w;
	if (params1.x > 0 && radiusM > 0) {
		float perM = 1 / (cb1[31].w * 1000);
		float groundN = radiusM * perM;
		float roomN = max(0.98 - groundN, 1e-4);
		float bottomN = groundN + params1.x * perM;
		float thickN = min(min(max(params1.y, 0) * perM, 0.9 * roomN), (2.0 / 3.0) * max(bottomN - s.rPlanet, 0));
		bottomN = min(bottomN, 0.98 - thickN);
		s.bottom = bottomN;
		s.top = bottomN + thickN;
		s.thickN = s.top > s.bottom ? thickN : 0;
		s.shellN = bottomN + 0.5 * thickN;
	}
	s.span = max(s.top - s.bottom, 1e-5);
	// The ground's radius (w507), or the atmosphere's base without one.
	s.groundN = radiusM > 0 ? radiusM / (cb1[31].w * 1000) : s.rPlanet;
	// Noise in units of the layer's height above the ground, so the clouds look the same size on
	// every planet: y503 = 8 makes a cloud about half the height. (Units of the distance to the
	// layer's horizon made a low layer's clouds hundreds of km wide, 2026-10-26.)
	s.k = params.y / 4 / max(s.shellN - s.rPlanet, 1e-4 * s.rPlanet);
	// A smooth veil (y530) takes the streaks away towards a sheet.
	float4 params30 = IniParams.Load(int2(530, 0));
	s.smooth = saturate(params30.y);
	s.halo = max(params30.z, 0);
	s.droplets = saturate(params30.w);
	s.squeeze = 1 / (1 + 6 * saturate(IniParams.Load(int2(504, 0)).y));
	if (s.smooth > 0)
		s.squeeze = 1 / (1 + 6 * saturate(IniParams.Load(int2(504, 0)).y) * (1 - s.smooth));
	// The wind z501 (m/s) over the clock z515 (s): a displacement d along x at the layer moves the
	// coordinate (up * shellN * k) by d k, squeezed as the coordinate is.
	float wind = max(params1.z, 0);
	float clock = IniParams.Load(int2(515, 0)).z;
	s.driftU = wind * clock / (cb1[31].w * 1000) * s.k;
	drift = float3(s.driftU * s.squeeze, 0, 0);
	evolve = wind > 0 ? clock / EVOLVE_S : 0;
	float4 params29 = IniParams.Load(int2(529, 0));
	s.wave = max(params29.x, 0);
	s.cells = max(params29.z, 0);
	s.rows = max(params29.w, 0);
	s.tail = max(params30.x, 0);
	// The threshold for the cover where the local time does not change it (slabThreshold).
	s.coverage = params.x;
	s.localTime = max(params1.w, 0);
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
	float3 warp = slabWarp(atPlain, p / r);
	float threshold = slabThreshold(slab, dot(p / r, -cb1[4].xyz));
	if (slab.wave > 0)
		threshold -= waveLift(slab.wave, p.x / r * slab.shellN * slab.k, slab.driftU, warp.x * 0.375, 0, threshold, WAVE_KEEP_SLAB);
	if (slab.cells > 0 || slab.rows > 0)
		threshold -= deckLift(slab, p / r, warp, dot(p / r, -cb1[4].xyz), 0, threshold);
	// The smooth veil's spread and its thinner extinction, the droplets' thinner density (0.43.143: the
	// canopy's ice and the lamps saw a denser, narrower veil than the one drawn).
	if (slab.smooth > 0) {
		threshold -= 0.3 * smoothSpread(slab, dot(p / r, -cb1[4].xyz));
		sigma /= pow(SMOOTH_THIN, slab.smooth);
	}
	float density = slabDensity(atPlain + warp, (r - slab.bottom) / slab.span, threshold, 4);
	if (slab.droplets > 0)
		density *= 1 - DROPLET_KEEP * slab.droplets;
	return density;
}

// ---- The slab's shadow on what lies under it (client 0.43.224) ----
//
// The share of the direct sunlight the slab takes from a point p (the planet's frame, atmosphere radii)
// with the sun towards `sun`: 1 - exp(-tau'), tau' the slab's optical depth along the way to the sun.
// 0 above the slab's top, with no slab and with no cloud on that way.
//
// tau is summed over SHADOW_TAPS samples of the march's own extinction (x504 is the alpha of a vertical
// path at density 1) along the ray's stretch between the slab's two spheres: with the sun 12 degrees up
// that stretch is 4.7 times the slab's thickness, so a cloud the camera sees through from above shades
// the ground under it well. The density is the cheap one of the march's own sun taps (slabDensityShadow:
// two octaves at most, by what the taps' spacing resolves; no erosion): a shadow is soft anyway, the
// sun's half degree alone blurs it over 175 m from 20 km up.
//
// tau' = (1 - SHADOW_FORWARD) tau: light a crystal scatters by a few degrees still falls on the ground
// under the cloud, as good as unscattered (the delta scaling of the two-stream methods: Joseph, Wiscombe
// and Weinman 1976, "The delta-Eddington approximation for radiative flux transfer", f = the phase
// function's second moment). For the march's two lobes (0.7 at g 0.75, 0.3 at g -0.2) that is
// 0.7 * 0.75^2 + 0.3 * 0.2^2 = 0.41. Without it a thin veil's shadow came out as dark as a thundercloud's:
// 31 % of the sun left under a cloud of optical depth 0.25 with the sun 12 degrees up, where 50 % is.
#define SHADOW_TAPS 4
#define SHADOW_FORWARD 0.41
float slabShadow(float3 p, float3 sun)
{
	float4 params = IniParams.Load(int2(503, 0));
	float opacity = IniParams.Load(int2(504, 0)).x;
	if (params.x <= 0 || opacity <= 0 || IniParams.Load(int2(505, 0)).z <= 0)
		return 0;
	Slab slab = loadSlab();
	float cb = dot(p, sun), cc = dot(p, p);
	if (cc >= slab.top * slab.top)
		return 0;
	// From inside the top sphere the ray leaves it once, at t1; from under the slab it enters the slab
	// where it leaves the bottom sphere.
	float t1 = -cb + sqrt(max(cb * cb - (cc - slab.top * slab.top), 0));
	float t0 = 0;
	if (cc < slab.bottom * slab.bottom)
		t0 = -cb + sqrt(max(cb * cb - (cc - slab.bottom * slab.bottom), 0));
	float len = t1 - t0;
	if (len <= 0)
		return 0;
	float sigma = -log(1 - min(opacity, 0.999)) / slab.span;
	if (slab.smooth > 0)
		sigma /= pow(SMOOTH_THIN, slab.smooth);
	float lod = clamp(1 + log2(1 / max(2 * len / SHADOW_TAPS * slab.k, 1e-6)), 1, 2);
	float tau = 0;
	[unroll] for (int i = 0; i < SHADOW_TAPS; i++) {
		float3 ps = p + sun * (t0 + len * (i + 0.5) / SHADOW_TAPS);
		float r = length(ps);
		float h = (r - slab.bottom) / slab.span;
		// A ray that starts inside the slab and dips under its base (a low sun seen from a mountain in
		// the cloud) has samples under it: empty.
		if (h <= 0 || h >= 1)
			continue;
		float3 up = ps / r;
		float3 atPlain = cloudCoord(ps, slab.k, slab.shellN, slab.squeeze, 0);
		float3 warp = slabWarp(atPlain, up);
		float day = dot(up, sun);
		float threshold = slabThreshold(slab, day);
		if (slab.wave > 0)
			threshold -= waveLift(slab.wave, ps.x / r * slab.shellN * slab.k, slab.driftU, warp.x * 0.375, 0, threshold, WAVE_KEEP_SLAB);
		if (slab.cells > 0 || slab.rows > 0)
			threshold -= deckLift(slab, up, warp, day, 0, threshold);
		if (slab.smooth > 0)
			threshold -= 0.3 * smoothSpread(slab, day);
		tau += slabDensityShadow(atPlain + warp, h, threshold, lod);
	}
	if (slab.droplets > 0)
		tau *= 1 - DROPLET_KEEP * slab.droplets;
	tau *= sigma * len / SHADOW_TAPS * (1 - SHADOW_FORWARD);
	return 1 - exp(-tau);
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
	// A quarter of a pixel on: the pass's buffer is half the depth's size, so a texel's middle lies on
	// the line between two of the depth's pixels, and which of the two a float's rounding gave changed
	// from texel to texel along winding lines all over the picture (12 % of the texels of
	// `ed_frame60273.rdc`). The pixel itself was as good either way, but the step from one texel's
	// pixel to the next (the dust's `pixelsPerTexel`) was 1 or 3 there, not 2, and every pixel the dust
	// reads round its own (the light on the ground, the terrain's plane) was looked for along a ray
	// half or half again as far out as it lies.
	pixel = int2(min(uv * float2(width, height) + 0.25, float2(width - 1, height - 1)));
	float depth = alt ? DepthAlt.Load(int3(pixel, 0)).x : Depth.Load(int3(pixel, 0)).x;
	dir = normalize(float3(dot(cb1[19].xyz, v1), dot(cb1[20].xyz, v1), dot(cb1[21].xyz, v1)));
	// The pass's own surface, the atmosphere's shell: the clip w of the dome's vertex, or, run over
	// the whole picture without the dome (edcas_sky_vs.hlsl gives v0.z < 0, 2026-11-08), the shell's
	// view depth worked out here, in metres by the rows' scale (atmosphere radii per metre), along
	// the view's forward axis. Elite draws the dome's back faces (it culls the front ones,
	// ed_frame1390264), so its surface is the ray's far crossing of the unit sphere round the
	// planet: the exit from inside, and from outside the far side, behind the planet's ground. Until
	// client 0.43.259 the near crossing was taken from outside: the ground lay behind it and counted
	// as nothing drawn. A ray that misses the sphere (from orbit, past the planet) has no surface:
	// anything the depth buffer holds nearer than Elite's sky (1e17 m, ed_frame1390264) is drawn
	// there, a ship against the haze above the atmosphere.
	float own = v0.z;
	if (own < 0) {
		float3 c = cb1[3].xyz;
		float cd = dot(c, dir), cc = dot(c, c);
		float disc = cd * cd - cc + 1;
		float t = disc > 0 ? -cd + sqrt(disc) : 0;
		own = t > 0 ? t / length(cb1[19].xyz) * dot(normalize(v1), cb1[279].xyz) : 1e16;
	}
	if (depth >= own)
		return -1;
	// The ray's own length, by the rows' scale: not the hit turned into the planet's frame minus the
	// camera's place there. Both are numbers near 1 in atmosphere radii, which a float holds to 0.2 m
	// on a 3,578 km planet, so that distance was off by up to as much, in blocks over the picture
	// that shift as the camera moves: the dust's streamers, some centimetres wide, were drawn as
	// chains of offset blocks (the owner's clip of 2026-11-17, 16:03: "pixelated stripes which move
	// with the camera"; `ed_frame29156.rdc`: beaded streamers, smooth lines with this).
	return length(v1 / dot(v1, cb1[279].xyz) * depth) * length(cb1[19].xyz);
}

// A pixel's view ray as viewRay gives it. The frame's one cloud pass (edcas_clouds_final.hlsl) works it out
// once and hands it to each of its layers: each used to call viewRay itself, six copies in the compiled
// pass, 0.013 ms apiece on ed_frame66376 (2026-11-12, client 0.43.145).
// The sun's share on what the depth buffer's `pixel` shows, by Elite's shadow (SunShade): 1 in the sun,
// 0 in shadow; -1 where there is none to be had (the texture not bound: an older edcas.ini, a frame
// without the sun pass). The texture is the picture's size, as the depth buffer; were it not, the
// pixel is taken at the same place of it.
float sunShade(int2 pixel)
{
	uint width, height, depthWidth, depthHeight;
	SunShade.GetDimensions(width, height);
	if (width == 0)
		return -1;
	if (asint(cb1[25].w) == 1)
		DepthAlt.GetDimensions(depthWidth, depthHeight);
	else
		Depth.GetDimensions(depthWidth, depthHeight);
	float2 at = (float2(pixel) + 0.5) * float2(width, height) / float2(depthWidth, depthHeight);
	return SunShade.Load(int3(clamp(int2(at), 0, int2(width, height) - 1), 0));
}

struct Ray
{
	float3 dir;    // in the planet's frame
	int2 pixel;    // of the depth buffer
	float ground;  // how far along it something is drawn, atmosphere radii; -1 for nothing
};

Ray rayOf(float3 v0, float3 v1)
{
	Ray ray;
	ray.ground = viewRay(v0, v1, ray.dir, ray.pixel);
	return ray;
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

void CLOUDS_MAIN(float3 v0, float3 v1, float3 v2, float3 v3, Ray ray, out float4 o0)
{
	// A pixel's angle (radians), before any branch: the footprint the waves are filtered to.
	float pixelAngle = length(fwidth(v1)) / max(length(v1), 1e-20);
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
	float3 dir = ray.dir;
	int2 pixel = ray.pixel;
	float groundN = ray.ground;
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
		float k = slab.k, squeeze = slab.squeeze;
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
		if (cameraR < bottom) {
			// From below, by the ray's elevation, but only where the way up to the slab's base is long,
			// as from above (client 0.43.226). By the elevation alone the clouds over the horizon came
			// on all at once as the camera rose into the slab: a metre under its base those rays are
			// faded, a metre over it they are inside, unfaded (`ed_frame62725.rdc` with the base moved
			// to 2 m over the camera and to 2 m under it: up to 41 of 255 on 7.4 % of the picture, the
			// band over the horizon). Near the base every ray reaches it within a few kilometres.
			float far = smoothstep(1, 3, t0 / max(1 - rPlanet, 1e-6));
			fade5 = lerp(1, smoothstep(-0.05, 0.15, elevation5), far);
		} else if (cameraR > slabTop) {
			// Only where that stretch is long: by the way to the slab's top in atmosphere depths
			// (cb1[31]'s shell, 102 to 125 km on the bodies measured), none within one, in full from
			// three (client 0.43.223). By the angle alone the clouds went out as the camera rose
			// through the slab's top: a metre above it every ray towards the horizon grazes the top,
			// a metre below it the same rays are inside and not faded (`ed_frame52891.rdc`, the
			// camera at 0.9799999 atmosphere radii, the top at 0.98: a bright bank over the horizon;
			// `ed_frame62725.rdc`, at 0.9800002: none; in a replay of the second with the top 2 m
			// higher the bank is back, and so it is with this fade off). The one and three are not
			// measured: Elite's own haze, which this stands for, is not known here.
			float far = smoothstep(1, 3, t0 / max(1 - rPlanet, 1e-6));
			fade5 = lerp(1, smoothstep(0.02, 0.2, -dot(dir, normalize(p0 + dir * t0))), far);
		}
		if (fade5 <= 0)
			DISCARD;

		float3 sun = -cb1[4].xyz;

		// x504 is the alpha of a vertical path through the whole slab at density 1.
		float sigma = -log(1 - min(opacity, 0.999)) / span;
		if (slab.smooth > 0)
			sigma /= pow(SMOOTH_THIN, slab.smooth);
		float3 sunColour = starLightTint() * cb1[8].xyz / max(max(cb1[8].x, cb1[8].y), max(cb1[8].z, 1e-30));

		// Two lobes (forward with a silver lining, a little backward), scaled so that the average
		// over all directions is 1 and capped at 4 so that looking into the sun does not blow out.
		// The forward lobe's g is 0.75 since 0.43.126 (0.6 before): ice of a few microns scatters with
		// g 0.7 to 0.85 (CLOUDS-RESEARCH.md §5.3); within 20 degrees of the sun the cap holds it as
		// before, from 40 degrees on it is a quarter darker, so the glow keeps to the sun (§6.4 anchor 2).
		float cosSun = dot(dir, sun);
		float g1 = 0.75, g2 = -0.2;
		float hg = 0.7 * (1 - g1 * g1) / pow(max(1 + g1 * g1 - 2 * g1 * cosSun, 1e-4), 1.5)
		         + 0.3 * (1 - g2 * g2) / pow(max(1 + g2 * g2 - 2 * g2 * cosSun, 1e-4), 1.5);
		float phase = min(hg, 4);
		// The 22 degree halo of a smooth veil of water ice, seen through it from below: on the sunlight
		// the cloud scatters, not on its sky-lit share.
		bool haloOn = slab.halo > 0 && cameraR < bottom;
		float3 haloExtra = 0;
		if (haloOn)
			haloExtra = haloTint(cosSun, slab.halo, dot(p0 / cameraR, sun)) - 1;

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
		// The waves' footprint along this ray (noise units): the step, or the pixel at the ray's far end
		// in the slab, whichever is wider (one value per ray: worked out per sample, the extra arithmetic
		// in the loop changed the compiled march's rounding with the waves off).
		float waveFoot = max(2 * L / min(steps, 32) * k, t1 * pixelAngle * k);
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

		AirColumn air = loadAirColumn();
		float3 col = 0;
		float T = 1;
		float dayWeighed = 0;  // the sun's height where the cloud is seen, weighed as the colour (iridescence)
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
			float3 warp = slabWarp(atPlain, p / r);
			float3 at = atPlain + warp;
			// The sun's height at this sample: the cover follows the local time (slabThreshold).
			float day = dot(p / r, sun);
			// The waves, filtered to the step along the ray and to the pixel.
			float waveCover = 0;
			if (slab.wave > 0)
				waveCover = waveLift(slab.wave, p.x / r * shellN * k, slab.driftU, warp.x * 0.375, waveFoot, slabThreshold(slab, day), WAVE_KEEP_SLAB) / 0.3;
			// The deck's cells by night and rows by day. The cheap skip below takes the threshold without
			// them: they shape the cloud the skip lets through and never wake empty space (with them in
			// it, the rows let 0.5 ms more samples through on ed_frame511315).
			float skipThreshold = slabThresholdWaved(slab, day, waveCover);
			if (slab.cells > 0 || slab.rows > 0)
				waveCover += deckLift(slab, p / r, warp, day, waveFoot, slabThreshold(slab, day)) / 0.3;
			// The smooth veil's spread, after the skip too (its 0.06 or so lies inside the skip's margin
			// of 0.13; before it, doubling the cover cost 0.6 ms on ed_frame511315).
			if (slab.smooth > 0)
				waveCover += smoothSpread(slab, day);
			float threshold = slabThresholdWaved(slab, day, waveCover);

			// Cheap skip: two octaves are far below the threshold's start, so five would be too.
			// Those two (fbm's first two noises, as `fbm(at, 2)` sums them) go on into the density.
			float lowCut = -0.25 * (1 - slabProfile(h));
			float n0 = noiseD(at, 1), n1 = noiseD(at * 2.03 + 17.1, 2.03);
			if ((0.5 * n0 + 0.25 * n1) / 0.75 + lowCut < skipThreshold - 0.03 - 0.1)
				continue;
			float base = slabDensityLod(at, h, threshold, lod, n0, n1);
			if (base <= 0)
				continue;
			float erosionWeight = saturate(lod - 3);
			if (slab.smooth > 0)
				erosionWeight *= 1 - SMOOTH_EROSION * slab.smooth;
			// Droplets do not fall out in wisps: softer edges, less erosion (w530).
			if (slab.droplets > 0)
				erosionWeight *= 1 - DROPLET_EROSION * slab.droplets;
			float erosion = 0;
			[branch] if (erosionWeight > 0)
				erosion = fbmD(at * 4.1 + float3(5.2, 1.3, 8.7) + (p / r) * (4 * evolve), 2, 4.1) * erosionWeight;
			float density = saturate(base - (1 - base) * erosion * 0.5 * (1 + edges));
			// The droplets' softer edges keep the layer's mean: thinner where the erosion acts.
			if (slab.droplets > 0)
				density *= 1 - DROPLET_KEEP * saturate(lod - 3) * slab.droplets;
			if (density <= 0)
				continue;

			// The same day / horizon logic as the shell, at this sample's own up and radius.
			float sky;
			float light = sunlightAt(day, rPlanet / r, twilightPower((r - slab.groundN) * cb1[31].w * 1000), sky);
			float3 tint = airTintOf(air, day, (r - slab.groundN) * cb1[31].w * 1000, float3(1.0, 0.55, 0.35));

			// Sunlight left after the cloud between this sample and the sun: a few steps, longer
			// and longer, with a cheaper density.
			float tau = 0, travel = 0;
			// Three steps of up to 2 octaves (four of 3 before; the shadow is soft anyway, and it
			// cost), the second octave faded by what the tap's length resolves (slabDensityShadow).
			// Skipping them without sunlight (`light` 0) saved nothing measurable (PERFORMANCE.md C5).
			// None in a smooth veil: a sheet with a sixth of the extinction barely shades itself, and its
			// twice the cloud cost 0.3 ms more in taps on ed_frame511315.
			[unroll] for (int s = 0; s < 3; s++) {
				float len = (s == 0 ? 0.15 : s == 1 ? 0.3 : 0.55) * thickN;
				travel += len;
				float3 ps = p + sun * travel;
				float hs = (length(ps) - bottom) / span;
				if (hs > 0 && hs < 1 && slab.smooth <= 0) {
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
			if (haloOn)
				radiance += sunColour * (0.5 * brightness) * (sunlit * light * phase) * haloExtra * tint;

			float stepAlpha = 1 - exp(-sigma * density * dtStep);
			col += T * stepAlpha * radiance;
			dayWeighed += T * stepAlpha * day;
			T *= 1 - stepAlpha;
			if (T < 0.01)
				break;
		}

		// Fall streaks (x530) hang under the base, in the shell between it and the tails' bottom: their
		// own VIRGA_SAMPLES samples, jittered as the march's, over the ray's stretch in that shell (one
		// or two pieces: in and out of it), laid over the slab's light after the march (they lie all
		// under it: in front of it from below, behind it from above or inside). The slab's march is
		// the one with no tails. Stretched down over the tails' length, it spent about half its
		// steps there, +0.44 to +0.54 ms on ed_frame511315 (0.43.134).
		if (slab.tail > 0) {
			float tailN = slab.tail * thickN;
			float tailBottom = max(bottom - tailN, rPlanet);
			float3 colTail = 0;
			float tTail = 1;
			float dB = cb * cb - (cc - bottom * bottom);
			float tEnd = groundN >= 0 ? groundN : 1e30;
			[branch] if (dB > 0 && tailBottom < bottom) {
				float sB = sqrt(dB);
				float a1 = max(-cb - sB, 0), b2 = min(-cb + sB, tEnd);
				// Without the tails' bottom sphere on the ray, one piece; with it, the parts before and after.
				float b1 = b2, a2 = b2;
				float dT = cb * cb - (cc - tailBottom * tailBottom);
				if (dT > 0) {
					float sT = sqrt(dT);
					b1 = clamp(-cb - sT, a1, b2);
					a2 = clamp(-cb + sT, b1, b2);
				}
				// Bounded at 10 slab thicknesses of the shell along the ray, as the march is.
				float len1 = b1 - a1, lenT = min(len1 + b2 - a2, 10 * thickN);
				// More along a long stretch (a grazing ray), so a sample stays under half a noise unit of the
				// heads, as the march's steps do.
				int nT = clamp(int(ceil(lenT * k * 2)), VIRGA_SAMPLES, VIRGA_MAX_SAMPLES);
				float dtT = lenT / nT;
				[loop] for (int j = 0; j < nT && lenT > 0; j++) {
					float along = (j + jitter) * dtT;
					float3 p = p0 + dir * (along < len1 ? a1 + along : a2 + along - len1);
					float r = length(p);
					float below = bottom - r;
					if (below <= 0 || below >= tailN)
						continue;
					float head;
					float tailDensity = virgaDensity(slab, p, below, tailN, sun, head);
					if (tailDensity <= 0)
						continue;
					float dayT = dot(p / r, sun);
					float skyT;
					float lightT = sunlightAt(dayT, rPlanet / r, twilightPower((r - slab.groundN) * cb1[31].w * 1000), skyT);
					float sunlitT = exp(-sigma * head * span * 0.5);
					float3 radianceT = sunColour * (0.5 * brightness) * (sunlitT * lightT * phase + 0.25 * skyT * 0.4) * airTintOf(air, dayT, (r - slab.groundN) * cb1[31].w * 1000, float3(1.0, 0.55, 0.35));
					float alphaT = 1 - exp(-sigma * tailDensity * dtT);
					colTail += tTail * alphaT * radianceT;
					tTail *= 1 - alphaT;
				}
			}
			col = cameraR < bottom ? colTail + tTail * col : col + T * colTail;
			T *= tTail;
		}
		float alpha = (1 - T) * fade5 * away5;
		if (alpha <= 0)
			DISCARD;
		// The blend is SRC_ALPHA INV_SRC_ALPHA, so the colour goes out straight, not premultiplied.
		float3 straight = col / max(1 - T, 1e-4);
		// Iridescence on the final colour, as on the shell (the hue follows the angle to the sun
		// and how dense the cloud is).
		float irid = iridescence * alpha * smoothstep(0.906, 0.985, cosSun) * dawnOnly(dayWeighed / max(1 - T, 1e-4));
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
	float3 warp = float3(fbmD(at * 0.5 + 3.7 + up * evolve, 3, 0.5), noiseD(at * 0.5 + 31.7 + up * evolve, 0.5), noiseD(at * 0.5 + 57.3 + up * evolve, 0.5)) - 0.5;
	at += warp * 1.6;

	float f = fbmD(at, 5, 1);
	float threshold = slabThreshold(slab, dot(up, -cb1[4].xyz));
	if (slab.smooth > 0)
		threshold = slabThresholdWaved(slab, dot(up, -cb1[4].xyz), smoothSpread(slab, dot(up, -cb1[4].xyz)));
	if (slab.wave > 0)
		threshold -= waveLift(slab.wave, up.x * slab.shellN * slab.k, slab.driftU, warp.x * 1.6 * 0.375,
		                      t / top * pixelAngle * slab.k / max(abs(dot(dir, up)), 0.05), threshold, WAVE_KEEP_SHELL);
	float base = smoothstep(threshold - 0.03, threshold + 0.12, f);
	if (base <= 0)
		DISCARD;

	// Erosion: fine noise bites into the thin parts the most, which leaves wispy edges. Half
	// strength: at full strength (and the narrower threshold above) the clouds averaged a sixth
	// of the density of the first version and vanished in game (2026-10-26).
	float erosion = fbmD(at * 4.1 + float3(5.2, 1.3, 8.7) + up * (4 * evolve), 3, 4.1);
	if (slab.smooth > 0)
		erosion *= 1 - SMOOTH_EROSION * slab.smooth;
	if (slab.droplets > 0)
		erosion *= 1 - DROPLET_EROSION * slab.droplets;
	float density = saturate(base - (1 - base) * erosion * 0.5);
	// Seen from far out the clouds would be finer than a pixel: they fade out up to y505 planet
	// radii above the ground (from a third of it on; 0 = the default 20). 1.5 radii hid them on
	// a supercruise approach until close; 20 looked much better in game (2026-10-26).
	float reach = IniParams.Load(int2(505, 0)).y;
	reach = reach > 0 ? reach : 20;
	float away = 1 - smoothstep(reach / 3, reach, (length(pos) - planet) / planet);
	float a = density * fade * away * saturate(opacity);
	if (slab.smooth > 0)
		a /= pow(SMOOTH_THIN, slab.smooth);
	if (a <= 0)
		DISCARD;

	// Sunlight at the hit point. The sun is still seen from there while it is not lower than the
	// horizon of the cloud shell, which lies below the horizon of the ground. So the clouds stay
	// lit a little after the ground goes dark (lit), and the part lit by day is as before.
	float3 sun = -cb1[4].xyz;
	float day = dot(up, sun);
	float sky;
	float light = sunlightAt(day, planet / shell, twilightPower(shell * 1000 - IniParams.Load(int2(507, 0)).w), sky);
	float3 tint = airTint(day, shell * 1000 - IniParams.Load(int2(507, 0)).w, float3(1.0, 0.55, 0.35));
	// A second sample a little towards the sun darkens the sides facing away from it.
	float shade = 1 - 0.6 * saturate(fbmD(at + sun * 0.4, 5, 1) - f + 0.1);

	// Iridescence: within about 25 degrees of the sun the tint shifts to a pastel palette whose
	// hue follows the angle to the sun (bands) and the cloud's own noise.
	float cosSun = dot(dir, sun);
	float irid = iridescence * a * smoothstep(0.906, 0.985, cosSun) * dawnOnly(day);
	if (irid > 0) {
		float hue = acos(saturate(cosSun)) * 7 + (f - 0.5) * 1.2 + erosion * 0.4;
		float3 pastel = 1 + 0.35 * cos(6.2831853 * (hue + float3(0, 0.33, 0.67)));
		tint = lerp(tint, pastel, irid);
	}

	// Sunlit ice in the sun's colour (cb1[8], normalised). The pass's target is already exposed,
	// 1 being white, so brightness 1 is ice at half of white in full sun. cb1[8] x exposure itself
	// came to 1e-4..1e-2, too small for the clouds' own scale, 2026-10-26.
	float3 sunColour = starLightTint() * cb1[8].xyz / max(max(cb1[8].x, cb1[8].y), max(cb1[8].z, 1e-30));
	if (slab.halo > 0 && c < 0)
		sunColour *= haloTint(cosSun, slab.halo, dot(normalize(cb1[3].xyz), sun));
	float3 radiance = sunColour * (0.5 * brightness) * light * shade * tint;
	o0 = float4(radiance, inBand(a, band));
#ifdef EDCAS_FINAL
	o0.rgb *= o0.a;  // into the half-size buffer, premultiplied for its upscaling
#endif
}

// ---- The second layer: the main gas's high clouds over a trace gas's layer (since 0.43.127) ----
//
// Row 521 from EDCAS: x521 the cover, y521 the height over the ground (metres; 5 scale heights, held
// under the atmosphere's top), z521 the alpha of a vertical path through a dense patch (vertical
// optical depth 0.04 to 0.10 at the default strength: faint by day), w521 their wind (m/s). 0 cover =
// none. Mars has both, CO2 clouds at 60 to 85 km over its water belt (CLOUDS-RESEARCH.md §3.1, §6.2).
// Drawn as a thin shell (no second march, CLOUDS-PLAN.md §3), with the slab's noise, drift and local
// time (amplitude 1.5: they belong to dawn, dusk and night), the twilight of their height and the
// iridescence of fresh high clouds at dawn and dusk. A shell's alpha grows with the slant path, the
// anchor that shows such faint layers at all (§5.1, §6.4 anchor 1: a vertical 0.05 is seen at the
// horizon), and the forward lobe makes them glow near the sun (anchor 2). Premultiplied; inFront when
// the camera is above them (then they lie in front of the slab).
float4 highClouds(float3 v0, float3 v1, Ray ray, out bool inFront)
{
	inFront = false;
	float pixelAngle = length(fwidth(v1)) / max(length(v1), 1e-20);
	float4 p21 = IniParams.Load(int2(521, 0));
	float radiusM = IniParams.Load(int2(507, 0)).w;
	if (p21.x <= 0 || p21.z <= 0 || radiusM <= 0)
		return 0;
	float3 dir = ray.dir;
	int2 pixel = ray.pixel;
	float groundN = ray.ground;
	float mPerUnit = cb1[31].w * 1000;
	float rPlanet = cb1[31].z / cb1[31].w;
	float shellN = min(radiusM / mPerUnit + max(p21.y, 0) / mPerUnit, 0.98);
	float3 cam = cb1[3].xyz;
	float b = dot(cam, dir);
	float c = dot(cam, cam) - shellN * shellN;
	float disc = b * b - c;
	if (disc < 0)
		return 0;
	float t = c < 0 ? -b + sqrt(disc) : -b - sqrt(disc);
	if (t <= 0 || (groundN >= 0 && t >= groundN))
		return 0;
	inFront = c > 0;
	float3 up = (cam + t * dir) / shellN;
	// The horizon fade of the shell, and the distance fade from orbit (y505), as the main layer's.
	float fade = c < 0 ? smoothstep(-0.02, 0.25, dot(dir, normalize(cam))) : smoothstep(0.02, 0.25, -dot(dir, up));
	float reach = IniParams.Load(int2(505, 0)).y;
	reach = reach > 0 ? reach : 20;
	float away = 1 - smoothstep(reach / 3, reach, (length(cam) - rPlanet) / rPlanet);
	if (fade * away <= 0)
		return 0;

	// Noise in layer heights, as the slab's, streaky (y504 1: thin high clouds are filamentary,
	// §5.4), drifting with their own wind.
	float k = IniParams.Load(int2(503, 0)).y / 4 / max(shellN - rPlanet, 1e-4 * rPlanet);
	float squeeze = 1.0 / 7.0;
	float clock = IniParams.Load(int2(515, 0)).z;
	drift = float3(max(p21.w, 0) * clock / mPerUnit * k * squeeze, 0, 0);
	evolve = p21.w > 0 ? clock / EVOLVE_S : 0;
	float3 at = up * (shellN * k);
	at.x *= squeeze;
	float3 warp = float3(fbmD(at * 0.5 + 3.7 + up * evolve, 3, 0.5), noiseD(at * 0.5 + 31.7 + up * evolve, 0.5), noiseD(at * 0.5 + 57.3 + up * evolve, 0.5)) - 0.5;
	at += warp * 1.6;
	float3 sun = -cb1[4].xyz;
	float day = dot(up, sun);
	float local = 1 + 1.5 * ((1.5 - 0.9 * saturate(day)) / 1.05 - 1);
	float threshold = 0.5 + 0.3 * (0.5 - p21.x * max(local, 0));
	// The pixel's footprint on the shell (noise units), for the waves and the noise.
	float foot = t * pixelAngle * k / max(abs(dot(dir, up)), 0.05);
	// Their waves (y529), as the main layer's.
	float wave = IniParams.Load(int2(529, 0)).y;
	if (wave > 0)
		threshold -= waveLift(wave, up.x * shellN * k, max(p21.w, 0) * clock / mPerUnit * k, warp.x * 1.6 * 0.375,
		                      foot, threshold, WAVE_KEEP_SHELL);
	// The noise filtered to the footprint, the edge widened by the spread that took away (fbmFoot).
	float lost;
	float f = fbmFoot(at, 5, 1, foot, lost);
	float base = smoothstep(threshold - 0.03 - 2 * lost, threshold + 0.12 + 2 * lost, f);
	if (base <= 0)
		return 0;
	float lostErosion;
	float erosion = fbmFoot(at * 4.1 + float3(5.2, 1.3, 8.7) + up * (4 * evolve), 3, 4.1, foot * 4.1, lostErosion);
	float density = saturate(base - (1 - base) * erosion * 0.5);
	// Slant path: the vertical optical depth over the cosine of the angle the ray meets the shell at.
	float mu = max(abs(dot(dir, up)), 0.1);
	float a = (1 - exp(log(1 - min(p21.z, 0.999)) * density / mu)) * fade * away;
	if (a <= 0)
		return 0;

	float sky;
	float light = sunlightAt(day, rPlanet / shellN, twilightPower(max(p21.y, 0)), sky);
	float3 tint = airTint(day, p21.y, float3(1.0, 0.55, 0.35));
	float cosSun = dot(dir, sun);
	float hg = 0.7 * (1 - 0.75 * 0.75) / pow(max(1 + 0.75 * 0.75 - 2 * 0.75 * cosSun, 1e-4), 1.5)
	         + 0.3 * (1 - 0.04) / pow(max(1 + 0.04 + 2 * 0.2 * cosSun, 1e-4), 1.5);
	float irid = 0.5 * a * smoothstep(0.906, 0.985, cosSun) * dawnOnly(day);
	if (irid > 0) {
		float hue = acos(saturate(cosSun)) * 7 + (f - 0.5) * 1.2 + erosion * 0.4;
		tint = lerp(tint, 1 + 0.35 * cos(6.2831853 * (hue + float3(0, 0.33, 0.67))), irid);
	}
	float brightness = IniParams.Load(int2(503, 0)).w;
	float3 sunColour = starLightTint() * cb1[8].xyz / max(max(cb1[8].x, cb1[8].y), max(cb1[8].z, 1e-30));
	float3 radiance = sunColour * (0.5 * brightness) * (light * min(hg, 4) + 0.25 * sky) * tint;
	return float4(radiance * a, a);
}

// ---- Photochemical haze (since 0.43.128) ----
//
// Methane, ammonia and sulphur dioxide with water make a haze in sunlight that does not condense
// (CLOUDS-RESEARCH.md §4.2): tholins orange-brown with nitrogen (Titan, Pluto), hydrocarbons
// yellow-grey from methane alone, hydrazine white-yellow from ammonia, sulphur pale yellow. It is
// global, its density falls with the air's scale height, its vertical optical depth is 0.01 to 0.1:
// nearly invisible overhead, a band at the horizon and at the limb from orbit, bright towards the sun
// (§5.1, §6.2, §6.4 anchors 1 and 2). Row 522 from EDCAS: x522 its vertical optical depth from the
// ground (0 = none), y522 the scale height (m), z522 the ground's radius (m), w522 how blue it turns
// in forward scatter (tholins at the limb, §5.2); x528 y528 z528 its colour. The optical depth along
// a view ray is closed-form (no march): an exponential atmosphere's column from a point to infinity is
// its vertical column there times the Chapman function, here in its form for a radius of many scale
// heights, sqrt(pi x / 2) erfcx(cos sqrt(x / 2)) (x the point's radius in scale heights, erfcx(z) =
// exp(z^2) erfc(z)): within 2 % of a numeric integral from the zenith to the horizon on a body of 400
// scale heights of radius. A ray that ends on the ground is taken from the ground back to the camera,
// upwards, where the function is stable (downwards it is a difference of two terms of exp(R / H)).
// Until client 0.43.200 it was sqrt(x) / (sqrt(x) cos + 1), after Schüler (GPU Pro 3, 2012): a
// fifth too little along the horizon, where the haze shows most, and so not of a piece with the
// stretches taken by their length (hazeColumn).

// (erfcx and chapmanColumn stand further up, with the sunlight's filter, which uses them too.)

#define HAZE_SPHERE_NEAR_M 10000
#define HAZE_SPHERE_FAR_M 40000
// The smooth sphere stands in for far ground only from high up (the camera's height over the haze's
// ground level, metres): near the ground the land in view rises far over that level (a basin's wall),
// and the sphere's hit lay behind it.
#define HAZE_SPHERE_LOW_M 20000
#define HAZE_SPHERE_HIGH_M 50000
// The haze's ground level is the land's own near the ground and the planet's radius from high up (the
// camera's height over the land's level, metres).
#define HAZE_LOCAL_NEAR_M 15000
#define HAZE_LOCAL_FAR_M 40000

// The lowest ground in view, smoothed (edcas_ground_ref.hlsl): the haze's ground level here, the
// fog's and the dust's in edcas_fog.hlsl and edcas_dust.hlsl.
Texture2D<float4> GroundRef : register(t111);

// The haze's column along the ray from `cam` in `dir` between `s0` and `s1` (atmosphere radii along
// it; `s1` over 1e29 for out to space), a stretch that lies outside the sphere of the haze's ground
// level `rG`: in vertical columns from that level. `H` the scale height. The Chapman function gives
// the column from a point upwards, so a stretch that climbs all the way is the lower end's less the
// upper end's, one that falls all the way the same taken from its far end back, and one that passes
// the ray's lowest point both halves upwards from that point.
float hazeOutside(float3 cam, float3 dir, float s0, float s1, float rG, float H)
{
	if (s1 <= s0)
		return 0;
	float X = rG / H;
	float3 p0 = cam + dir * s0;
	float r0 = length(p0);
	float h0 = max(r0 - rG, 0) / H, c0 = dot(dir, p0) / r0;
	if (s1 > 1e29)
		return chapmanColumn(X, h0, c0);
	float3 p1 = cam + dir * s1;
	float r1 = length(p1);
	float h1 = max(r1 - rG, 0) / H, c1 = dot(dir, p1) / r1;
	if (c0 >= 0)
		return max(chapmanColumn(X, h0, c0) - chapmanColumn(X, h1, c1), 0);
	if (c1 <= 0)
		return max(chapmanColumn(X, h1, -c1) - chapmanColumn(X, h0, -c0), 0);
	float hLow = max(length(cam - dir * dot(cam, dir)) - rG, 0) / H;
	return max(2 * chapmanColumn(X, hLow, 0) - chapmanColumn(X, h0, -c0) - chapmanColumn(X, h1, c1), 0);
}

// The haze's column along the ray from `cam` in `dir`, `end` long (atmosphere radii; negative for out
// to space), in vertical columns from its ground level `rG`: the haze thins with the height over that
// level by the scale height `H`, and under it it is as thick as at it (the land below the level, a
// basin under the planet's radius, is filled evenly, not by exp(depth / H): 10 times the dust 19 km
// down). So the ray is taken in stretches: outside the level's sphere by the Chapman function
// (hazeOutside), inside it by its length.
float hazeColumn(float3 cam, float3 dir, float end, float rG, float H)
{
	float far = end >= 0 ? end : 1e30;
	float b = dot(cam, dir), c = dot(cam, cam) - rG * rG, disc = b * b - c;
	// The stretch inside the sphere: none where the ray misses it or it lies behind the camera.
	float sIn = far, sOut = far;
	if (disc > 0) {
		float root = sqrt(disc);
		if (-b + root > 0) {
			sIn = min(max(-b - root, 0), far);
			sOut = min(-b + root, far);
		}
	}
	return hazeOutside(cam, dir, 0, sIn, rG, H) + (sOut - sIn) / H + hazeOutside(cam, dir, sOut, far, rG, H);
}

// How many samples the haze's column takes where it is summed (hazeInShell).
#define HAZE_SHELL_STEPS 16

// The haze's column as hazeColumn, but of a haze that ends at the top of Elite's atmosphere (the
// sphere of radius 1): its density is exp(-h / H) less what that is at the top, so it runs out to
// nothing there without a step, and times what keeps its vertical column from the ground level the
// same (x531, x522: the dust is all there, in the air the game has). Only the ray's stretch inside
// that sphere counts; a ray that misses it has no haze.
// The chain runs at Elite's sky draws alone, and from outside its atmosphere Elite draws the sky only
// while the shell is in view: a haze that reached past the shell came and went with a tilt of the
// camera, over the whole picture. On Outorns EH-S d5-2 A 2 b (0.48 m/s², 415 K, water vapour: a scale
// height of 397 km on a body of 426 km, the shell's top 92 km up) the dust haze had 67 % of its
// ground density at the ship 161 km up and lit all of space 120 times as bright as it was, and one
// frame later was gone (ed_frame110351, ed_frame131055, 2026-11-24).
// Where the shell is four scale heights deep or more, what is taken off is under 2 % of the density
// at the ground and the closed form stands (less the constant along the stretch). Under three, the
// difference would be of two nearly equal numbers, and the Chapman function is good to 2 % only for
// a radius of many scale heights: there the stretch is summed in HAZE_SHELL_STEPS samples, which a
// density that changes by less than e³ along it allows. Between, both, blended.
float hazeInShell(float3 cam, float3 dir, float end, float rG, float H)
{
	float b = dot(cam, dir), c = dot(cam, cam) - 1, disc = b * b - c;
	if (disc <= 0)
		return 0;
	float root = sqrt(disc);
	float s0 = max(-b - root, 0), s1 = -b + root;
	if (end >= 0)
		s1 = min(s1, end);
	if (s1 <= s0)
		return 0;
	float len = s1 - s0;
	float top = max(1 - rG, 0) / H;
	float eTop = exp(-top);
	// the vertical column of exp(-h / H) - eTop from the ground level to the top, in scale heights
	float whole = 1 - eTop * (1 + top);
	if (whole <= 1e-6)
		return 0;
	float3 from = cam + dir * s0;
	float deep = smoothstep(3, 4, top);
	float closed = 0, summed = 0;
	if (deep > 0)
		closed = max(hazeColumn(from, dir, len, rG, H) - eTop * len / H, 0);
	if (deep < 1) {
		float dt = len / HAZE_SHELL_STEPS;
		for (int i = 0; i < HAZE_SHELL_STEPS; i++) {
			float h = max(length(from + dir * ((i + 0.5) * dt)) - rG, 0) / H;
			summed += max(exp(-h) - eTop, 0);
		}
		summed *= dt / H;
	}
	return lerp(summed, closed, deep) / whole;
}

// A haze mixed through the air, thinning with the height by the scale height, along a pixel's view ray
// in front of the ground (or out to space), premultiplied. `p`: its vertical optical depth, its scale
// height (m), the ground's radius (m), how blue it turns in forward scatter; `colour`: its own.
float4 columnHaze(Ray ray, float4 p22, float3 colour)
{
	if (p22.x <= 0 || p22.y <= 0 || p22.z <= 0)
		return 0;
	float3 dir = ray.dir;
	int2 pixel = ray.pixel;
	float groundN = ray.ground;
	float mPerUnit = cb1[31].w * 1000;
	float H = p22.y / mPerUnit;              // the scale height, atmosphere radii
	float3 cam = cb1[3].xyz;
	float r0 = length(cam);
	// The haze's ground level. The planet's radius (the journal's) is the mean of its land: a basin's
	// floor lies far under it, and with the heights taken from that radius and held at 0 below it the
	// columns of a camera in the basin did not fit together. Over a floor 19 km under the radius, the
	// camera 8 km up and the wall 70 km off, a ray just over eye level got an optical depth of 0.37
	// and one just under it 0.58: a hard line across the wall at eye level, the wall's foot gone under
	// it and its top nearly clear, and the line climbed the wall as the ship did (ed_frame347774,
	// Plooe Thoi EP-R d4-9 C 4, 2026-11-17). So near the ground the level is the land's own, the
	// lowest ground in view (edcas_ground_ref.hlsl; the camera's own height where none is kept), no
	// higher than the radius: the haze lies in the basin and the wall rises out of it. From high up it
	// is the planet's radius, as it was: the land kept is the place the ship left.
	float rJournal = p22.z / mPerUnit;
	float rBase = cb1[31].z / cb1[31].w;
	float4 ref = GroundRef.Load(int3(0, 0, 0));
	float rLocal = min(rJournal, r0);
	if (ref.y > 0)
		rLocal = min(rLocal, rBase + ref.x / ref.y * (1 - rBase));
	float rGround = lerp(rLocal, rJournal, smoothstep(HAZE_LOCAL_NEAR_M, HAZE_LOCAL_FAR_M, (r0 - rLocal) * mPerUnit));
	// Far ground is taken as the smooth sphere of rGround, not the depth buffer's hit (HAZE_SPHERE_NEAR_M
	// to _FAR_M): the column lies within a few scale heights of the ground, so it follows the terrain's
	// height, and Elite redraws far terrain at another detail from frame to frame. From orbit the haze
	// over each tile jumped: blotches that shimmered and swept round the point under the ship
	// (ed_frame814252/3, 2026-11-16, a 0.05 dust haze of 6 km scale height: 892 pixels of the disc
	// changed between frames, 271 with the sphere, 218 without the haze). Only from high up
	// (HAZE_SPHERE_LOW_M), and only where the ray comes down on the sphere ahead.
	if (groundN >= 0) {
		float b = dot(cam, dir), c = dot(cam, cam) - rGround * rGround, disc = b * b - c;
		if (c > 0 && disc > 0 && b < 0)
			groundN = lerp(groundN, -b - sqrt(disc), smoothstep(HAZE_SPHERE_NEAR_M, HAZE_SPHERE_FAR_M, groundN * mPerUnit) * smoothstep(HAZE_SPHERE_LOW_M, HAZE_SPHERE_HIGH_M, (r0 - rGround) * mPerUnit));
	}
	float column = hazeInShell(cam, dir, groundN, rGround, H);
	// Where the haze along the ray is densest, which is where its light is taken: the ray's lowest
	// point on it, the camera for a ray that climbs and the ground for one that falls all the way. Until
	// client 0.43.200 a ray that ended on the ground took the camera from eye level up and the ground
	// under it: the light stepped at eye level (the ground 120 km off has another sun height and lies
	// 8 km lower), a faint straight line across the far wall (ed_frame347774).
	float3 mid = cam + dir * clamp(-dot(cam, dir), 0, groundN >= 0 ? groundN : 1e30);
	float tau = p22.x * column;
	float a = 1 - exp(-tau);
	// Faded out from far off, as the clouds (y505).
	float reach = IniParams.Load(int2(505, 0)).y;
	reach = reach > 0 ? reach : 20;
	float rPlanet = cb1[31].z / cb1[31].w;
	a *= 1 - smoothstep(reach / 3, reach, (r0 - rPlanet) / rPlanet);
	if (a <= 1e-4)
		return 0;

	float3 sun = -cb1[4].xyz;
	float rm = max(length(mid), 1e-6);
	float day = dot(mid / rm, sun);
	float sky;
	float light = sunlightAt(day, rGround / max(rm, rGround), twilightPower(max(rm - rGround, 0) * mPerUnit), sky);
	float cosSun = dot(dir, sun);
	// Small particles: a forward lobe (g 0.6) and some back light, normalised and capped as the clouds'.
	float hg = 0.8 * (1 - 0.36) / pow(max(1 + 0.36 - 1.2 * cosSun, 1e-4), 1.5) + 0.2;
	hg = min(hg, 4);
	// Tholins scatter blue forwards (Pluto's blue limb).
	colour = lerp(colour, float3(0.55, 0.7, 1.0), saturate(p22.w) * saturate((hg - 1) / 3));
	float3 sunColour = starLightTint() * cb1[8].xyz / max(max(cb1[8].x, cb1[8].y), max(cb1[8].z, 1e-30));
	// A thin layer's single scattering: omega P tau / 4 of a sunlit white surface (P averaging 1 over
	// all directions, omega near 1 for haze): 0.25 on the pass's scale, where 1 is white.
	float3 radiance = sunColour * 0.25 * colour * (light * hg + 0.25 * sky) * airTint(day, max(rm - rGround, 0) * mPerUnit, float3(1.0, 0.55, 0.35));
	return float4(radiance * a, a);
}

// The photochemical haze (rows 522 and 528, from EDCAS).
float4 photoHaze(float3 v0, float3 v1, Ray ray)
{
	float3 colour = float3(IniParams.Load(int2(528, 0)).xyz);
	return columnHaze(ray, IniParams.Load(int2(522, 0)), any(colour > 0) ? colour : float3(0.8, 0.6, 0.4));
}

// The dust haze: the fine dust the wind has lifted stays up for days to months and is mixed through
// kilometres of air (Mars: a boundary layer of 1 to 10 km and more, a column optical depth of 0.4 to
// 1.25 at Gale; reports/Planetary dust and wind rendering.md, L1), where the dust's far drift was a
// layer 15 m deep. In closed form as the photochemical haze: no march, so no rings from above, and no
// ground level to get wrong. x531 its vertical optical depth (0 = none), y531 the air's scale height
// (m), from EDCAS (planet_dust.rs); the ground's radius is the dust's w515, the colour its row 516.
// Along the horizon the column is some 14 times the vertical one on a body of 1300 km with a 10 km
// scale height: 0.05 is a clear veil there. It does not follow the hour or the fronts: only the sand
// near the ground does.
//
// The dust is held up by the air, and the air in the picture is Elite's, which thins by e every
// ELITE_AIR_M whatever the gas: its sky pass's table of the optical depth towards the sun (t2,
// README-ELITE.md), read for the sun at the zenith, falls by e every 7.5 to 8.0 km on five bodies
// whose own scale heights are 8.2, 10.3, 17.5, 45.8 and 397 km (renderdoc/airprobe.py, 2026-11-24:
// ed_frame60273, 31308, 893613, 150885, 54574). With the air's own scale height the dust of a body
// of light gravity stood far over the blue rim Elite draws: on Outorns EH-S d5-2 A 2 b (397 km,
// since client 0.43.264 thinned out to the shell's top 92 km up) a brown glow 30 to 100 km over a
// rim some 10 km deep (ed_frame54574, the owner: "the dust layer goes further than the visible
// blue atmosphere"). So the dust's scale height is the air's, but no more than Elite's.
#define ELITE_AIR_M 7900.0
float4 dustHaze(Ray ray)
{
	float4 p31 = IniParams.Load(int2(531, 0));
	float3 colour = float3(IniParams.Load(int2(516, 0)).xyz);
	return columnHaze(ray, float4(p31.x, min(p31.y, ELITE_AIR_M), IniParams.Load(int2(515, 0)).w, 0), any(colour > 0) ? colour : float3(0.62, 0.45, 0.32));
}
