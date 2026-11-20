// Wind-blown dust near the ground, for the frame's one cloud pass (edcas_clouds_final.hlsl).
// Included after edcas_fog.hlsl, whose ground level (GroundRef) it uses.
//
// Where there is air the wind moves the ground three ways, as on Mars:
// - Saltation: sand grains hop along the ground in sheets that stream with the wind. In thin
//   air and low gravity they hop about ten times as high as on Earth (Kok 2010, arXiv 1001.4840;
//   large-eddy simulations, EPJ E 2014), so the sheet here is about a metre deep.
// - Gusts: patches of stronger wind, tens to a hundred metres across, drift with the wind and
//   lift the fine dust a few metres in billows while they pass.
// Dust devils (Mars has them by day) are left out: towers hundreds of metres tall, seen from
// kilometres away, would need lighting and shadows this pass can't give them.
// Airless worlds have none of it: EDCAS writes no strength there. The dust a ship's thrusters
// blow up is the game's own: the volumetric landing dust that was here went with client 0.43.22
// (README-ELITE.md, "Landing dust"), and the redraw of the game's that followed it was removed
// 2026-11-03 (README.md, "Landing dust (removed 2026-11-03)").
//
// IniParams row 15: x515 strength (0 = none), y515 wind speed in m/s (0 = 8), z515 seconds
// (edcas.ini sets it from 3DMigoto's `time`), w515 the planet's radius in metres (from EDCAS; only
// whether it is set counts, the metres come from Elite's cb1[31]).
// Row 16: x516 y516 z516 the dust's colour (0 = Mars dust's ochre); w516 = 1 a debug view of the
// terrain mask: red where the mask is farther than the depth buffer, green where nearer (full
// at 5 %), black where they agree, blue where the mask has no ground; w516 = 2 the lit picture as
// the dust reads it (t119), which should look like the game without the mods' dust.
// Row 18: x518 DUST_BRIGHTER, y518 DUST_PALE, z518 DUST_THICK (0 = those), w518 times as thick far
// drift (0 = 1), to tune in game. Row 19: x519 the far drift's reach in metres (0 = FAR_REACH_M),
// y519 the sheet's scale height in metres (0 = SHEET_M; from EDCAS by the gravity), z519 the share of
// the ground a dust front lies over (0 = all of it: no fronts), w519 the night's dust over the day's
// (0 = as by day).
//
// t118: the terrain mask, the ground's distance from the camera in metres where Elite's ground
// shaders drew it (ground_tint.py mask, edcas.ini CommandListEdcasTerrainMask), 0 elsewhere.

Texture2D<float> TerrainDist : register(t118);  // the sun pass reads it too, for the sunset on the ground (star_light.py sunset_block)
// t119: the lit picture before the clouds go on (edcas.ini CommandListEdcasSky), for the light
// on the ground under the dust.
Texture2D<float4> Scene : register(t119);

#define DUST_STEPS 16
// How much brighter the dust is than the ground it is lifted from, both in the same light (x518):
// the fine fraction the wind lifts reflects more than the sand it lies on (Mars's bright dust has
// about twice the albedo of its dark sand). At 1, as bright as the ground, it could not be seen,
// nor at 1.4 while the picture was read wrongly; read rightly, 1.2 tuned in game (2026-10-29).
#define DUST_BRIGHTER 1.2
// How far the dust's colour goes towards a white of the same brightness (y518): none, tuned in
// game; at 0.3 it looked white (2026-10-29).
#define DUST_PALE 0.0
// How many times as thick (z518): 4 tuned in game, so it shows close by too (2026-10-29).
#define DUST_THICK 4.0
// The wind-blown dust is full up to DUST_NEAR_M from the camera and gone by DUST_FAR_M (metres):
// a metre-deep sheet that far off is too thin to see, and over the whole plain it lit the
// ground up against the sky and made the horizon's stair steps stand out (2026-10-29).
#define DUST_NEAR_M 150.0
#define DUST_FAR_M 400.0
// The saltation sheet's scale height and the billows', in metres.
#define SHEET_M 0.8
#define BILLOW_M 6.0
// The far drift: beyond the local dust, dust blown along low over the land in patches, as a
// windy day shows it towards the hills (2026-10-29): a layer FAR_H_M deep over the lowest ground,
// in patches of FAR_PATCH_M drifting with the wind, from FAR_FROM_M as far as FAR_REACH_M (x519),
// at FAR_DENSITY (1 / m) in a patch at the ground. At 5 km the ridges beyond the plain's
// horizon (2 km away from eye height on 7 c) had none (2026-10-29).
#define FAR_FROM_M 300.0
#define FAR_REACH_M 15000.0
#define FAR_H_M 15.0
#define FAR_PATCH_M 600.0
#define FAR_DENSITY 0.0004
#define FAR_STEPS 8
// The gust patches' size in metres.
#define GUST_M 90.0
// How far from the camera the billows are seen in full, and where they are gone, metres (groundDust).
#define DUST_SEEN_NEAR_M 600.0
#define DUST_SEEN_FAR_M 1500.0
// How many times as thick the billows are drawn from over them (groundDust's `above`): looking down a ray
// crosses 50 to 130 m of the layer, a camera inside it looks through 400 m, and the dust is lit as the
// ground under it, so at 1, the same dust as from the ground, the puffs were faint. A look, not physics: the
// owner's choice of 1, 2, 4 and 10 on `ed_frame169294.rdc` (2026-11-12); at 4 and over the sand towards the
// sun whites out.
#define DUST_ABOVE 2.0
// How wide the spiral of pixels is that the dust on the ground takes its light from (groundDust),
// pixels of the picture. The dust is lit by the light that falls there, not by how dark the ground
// under it happens to be. From 12 pixels it took the ground's patches along: over ground of dark
// and pale patches some tens of pixels wide the dust was dark on the dark ones and pale on the
// pale ones, so the picture kept the patches' contrast in full and lost only the fine detail,
// which is what a lens out of focus does and not what a veil does (the owner, 2026-11-17: "as if
// one needed glasses"; `ed_frame60273.rdc`, the ground's contrast with the dust over without it:
// 0.973 of the fine detail, 0.997 of the patches). From 64 pixels both go down alike (0.970 and
// 0.969): a veil. A shadow narrower than that gets some of the light beside it; the shadowed
// hollow of that frame shows no change at its edge.
#define DUST_LIGHT_PX 64.0
struct Dust {
	float strength, wind, flow, time, mPerUnit, rPlanet, sheetM, activity, night;
	float3 colour, up, along, east;
};

// False when there is no dust (or the radius w515 not set).
bool loadDust(out Dust d)
{
	float4 p15 = IniParams.Load(int2(515, 0));
	float4 p16 = IniParams.Load(int2(516, 0));
	d.strength = p15.x;
	d.wind = p15.y > 0 ? p15.y : 8;
	d.time = p15.z;
	d.rPlanet = cb1[31].z / cb1[31].w;
	// cb1[31] is in kilometres (.w the atmosphere's top, .z its base, 1310 km under 7 c's
	// 1325 km ground; frame analysis 2026-10-28): EDCAS's radius over .z / .w gave metres 1.15 %
	// too long, the terrain mask (Elite's metres) never matched, and the ground ended in a
	// ragged ring (2026-10-29).
	d.mPerUnit = cb1[31].w * 1000;
	d.colour = any(p16.xyz > 0) ? p16.xyz : float3(0.62, 0.45, 0.32);
	// The saltation sheet is shallow on heavy worlds and deep on light ones: its flux falls by e every 5
	// to 10 cm on Earth (Martin and Kok 2017), and hops are as high as the gravity is low (1 / g, an
	// extrapolation: nothing but Earth's was ever measured). EDCAS writes 0.1 m at 1 g, 0.26 on Mars,
	// 0.63 on 7 c, at most 2 (DUST-PLAN.md phase 3, client 0.43.149). Not set: SHEET_M, as before.
	float4 p19 = IniParams.Load(int2(519, 0));
	d.sheetM = p19.y > 0 ? p19.y : SHEET_M;
	d.activity = p19.z > 0 ? p19.z : 1;
	// The streamers' speed, m/s: the wind at the height of the sheet they run in, not the wind a man's
	// height up. They are the footprints of the eddies near the ground and go with the air there
	// (Baas and Sherman 2005), and by the log law that air moves at u* / 0.4 * ln(height / roughness):
	// with EDCAS's own numbers (planet_dust.rs: a roughness of 1 cm; the friction speed of a gust that
	// lifts dust, 1.2 m/s, or the threshold's where that is more: y515 is the threshold's wind at
	// 1.5 m, 0.08 of it its friction speed) and the sheet's scale height. 6.9 m/s on Earth's 0.1 m,
	// 9.8 on Mars's 0.26 m (13.6 with its threshold of 20.7 m/s), 10.9 on a body of 0.26 g.
	// Until client 0.43.214 they went at y515, the wind at 1.5 m (20.7 m/s on Mars, and at least 10 on
	// every body, set by eye before there were streamers). The one speed measured, 4.01 m/s on an
	// Earth beach (Baas and van den Berg 2018, the wind then not given), is what the same law gives
	// for an Earth sand wind (a friction speed of 0.4 m/s, a roughness of 2 mm: 3.9 m/s); EDCAS's
	// gust is three times as strong a wind as that on every body.
	// The gusts, the fronts and the far drift, metres above the sheet, keep 0.8 of y515.
	d.flow = 2.5 * max(1.2, 0.08 * d.wind) * log(max(d.sheetM, 0.05) / 0.01);
	d.night = p19.w > 0 ? p19.w : 1;
	// The wind blows east, round the planet's spin axis. That axis is the frame's y: the camera's place
	// gives the journal's latitude as asin(y) and its longitude as atan2(z, x) (29.99 and -128.21
	// degrees on Plooe Thoi EP-R d4-9 C 4, `ed_frame29156.rdc`, 2026-11-17), so east, towards rising
	// longitude, is up x y. A little of up x z is added: at the poles up x y is nothing, and with it
	// the direction is a smooth one all over the planet but for one point by each pole (1 degree off
	// east at the equator, 7 at 80 degrees).
	// Until client 0.43.212 the wind blew along a planet axis (x, or y where x stands upright), and the
	// streamers along the one of two axes the ground lies along most: a wind out of the north on a
	// third of the planet and out of the east on the rest, turning by a right angle along the edges of
	// a cube round the planet.
	d.up = normalize(cb1[3].xyz);
	d.east = normalize(cross(d.up, float3(0, 1, 0)) + 0.02 * cross(d.up, float3(0, 0, 1)));
	// The gusts, the fronts and the far drift are noises of the planet's own frame (fbmMoving), and their
	// drift has to be one direction all over it: a direction that turns with the camera's place turns the
	// whole drift (the wind's speed times the clock, kilometres after an hour) with it, and the patches
	// slide as the camera moves. So they drift along the planet axis nearest east, x or z either way.
	// It changes where the camera crosses one of four meridians (the patches are others from there on).
	// Up to 45 degrees off the streamers' wind; the patches are soft and tens of metres wide, and show
	// how much sand there is more than where it goes.
	d.along = abs(d.up.z) >= abs(d.up.x) ? float3(d.up.z > 0 ? -1 : 1, 0, 0) : float3(0, 0, d.up.x > 0 ? 1 : -1);
	return d.strength > 0 && p15.w > 0;
}

// Value noise on an integer hash, for the wind-blown dust's planet-centred cells: they number a
// million, and past some thousands of cells the float hash (edcas_noise.hlsl) runs out
// of precision and its cells show as blocks. The lattice point's coordinates are mixed by large
// odd primes and then scrambled (MurmurHash3's finaliser): PCG3D (Jarzynski & Olano 2020) did as
// well at twice the work, and the dust took 110 FPS down to 75 (2026-10-29).
float hashU(float3 i)
{
	int3 c = int3(i);
	uint h = asuint(c.x) * 73856093u ^ asuint(c.y) * 19349663u ^ asuint(c.z) * 83492791u;
	h ^= h >> 16;
	h *= 0x85ebca6bu;
	h ^= h >> 13;
	h *= 0xc2b2ae35u;
	h ^= h >> 16;
	return h * (1.0 / 4294967295.0);
}

float noiseU(float3 x)
{
	float3 i = floor(x);
	float3 f = x - i;
	f = f * f * (3.0 - 2.0 * f);
	return lerp(lerp(lerp(hashU(i + float3(0, 0, 0)), hashU(i + float3(1, 0, 0)), f.x),
	                 lerp(hashU(i + float3(0, 1, 0)), hashU(i + float3(1, 1, 0)), f.x), f.y),
	            lerp(lerp(hashU(i + float3(0, 0, 1)), hashU(i + float3(1, 0, 1)), f.x),
	                 lerp(hashU(i + float3(0, 1, 1)), hashU(i + float3(1, 1, 1)), f.x), f.y), f.z);
}

// fbm (n octaves) of noiseU seen through a pixel `foot` wide (in p's units): each octave fades
// to its mean as the pixel grows to half its lattice cell, so ground far off and seen flat, where
// one pixel spans metres, gets the noise's average instead of a different value in every pixel
// (the far hills sparkled, 2026-10-29). Each octave is turned against the one before (and the
// first against the axes): the landing dust's noise lies along its direction out and its time of
// leaving, and sharpened, cells along one lattice showed as big squares moving out (2026-10-29).
static const float3x3 TURN = float3x3(0.00, 0.80, 0.60, -0.80, 0.36, -0.48, -0.60, -0.48, 0.64);

float fbmTurned(float3 p, const int n, float foot)
{
	float sum = 0, amp = 0.5, total = 0;
	[unroll] for (int i = 0; i < n; i++) {
		p = mul(TURN, p);
		// A pixel half the cell wide or more sees the octave's mean, 0.5 exactly: no noise worked out for it
		// (PERFORMANCE.md C1, 2026-11-10: it was, and blended away).
		float w = 1 - smoothstep(0.25, 0.5, foot);
		float octave = 0.5;
		[branch] if (w > 0)
			octave = lerp(0.5, noiseU(p), w);
		sum += amp * octave;
		total += amp;
		p = p * 2.03 + 17.1;
		foot *= 2.03;
		amp *= 0.5;
	}
	return sum / total;
}

// The path (m) a ray runs through a sheet `depthM` deep over the ground where it ends, `hit`
// `ground` along it, meeting it at `slope` (the sine of the angle; groundSlope): the camera's height over the ground's plane there decides how much of the
// sheet it looks down through. That height is the ray's length times its angle to the ground,
// held at the grazing angle at least: taken over the ground below the camera instead, it was 0
// for every hit at eye level or above, and the sheet ended in a hard ring there that grew as
// the commander jumped (on foot, 7 c, 2026-10-29).
float sheetPath(Dust d, float3 dir, float3 hit, float ground, float depthM, float slope)
{
	float hCam = ground * d.mPerUnit * slope;
	// At most 60 m: at a grazing angle it would run on to the horizon.
	return min(depthM / slope, 60) * (1 - exp(-hCam / depthM));
}

// The sine of the angle at which a ray meets the ground: against the planet's up or the terrain's
// own normal (from the depth), whichever is steeper. With the planet's up alone a hill's face
// turned towards the camera was met at a grazing angle, ran 60 m through the sheet and turned
// white (2026-10-29); the terrain's normal is noisy along edges, so it only ever thins the sheet.
float groundSlope(float3 dir, float3 hit, float3 normal)
{
	return max(max(-dot(dir, normalize(hit)), -dot(dir, normal)), 0.01);
}

// Value noise at the lattice cell `i` (whole numbers) and the place `f` (0..1) in it, on the
// integer hash (noiseU).
float noiseCell(float3 i, float3 f)
{
	f = f * f * (3.0 - 2.0 * f);
	return lerp(lerp(lerp(hashU(i + float3(0, 0, 0)), hashU(i + float3(1, 0, 0)), f.x),
	                 lerp(hashU(i + float3(0, 1, 0)), hashU(i + float3(1, 1, 0)), f.x), f.y),
	            lerp(lerp(hashU(i + float3(0, 0, 1)), hashU(i + float3(1, 0, 1)), f.x),
	                 lerp(hashU(i + float3(0, 1, 1)), hashU(i + float3(1, 1, 1)), f.x), f.y), f.z);
}

// fbmTurned's filtering (unturned) of the noise at `pM`, a point in the planet's frame in metres, shifted by `shiftM`
// (the wind's drift, metres) and `extra` (in the noise's cells), the cells `cellInv` to the metre
// on each axis. A planet-centred point is some 1,300 km out, where a float holds only an eighth
// of a metre: the wind's 10 cm a frame added to it went under in the rounding, and the dust
// stood still or jumped ahead with the camera's own rounding as it moved (on foot, 2026-10-29).
// So the point alone is split into its whole cells and the place in its cell (exactly), and the
// drift goes onto that place, where a float holds it finely.
float fbmMoving(float3 pM, float3 shiftM, float3 cellInv, float3 extra, const int n, float foot)
{
	float sum = 0, amp = 0.5, total = 0;
	[unroll] for (int k = 0; k < n; k++) {
		float3 big = pM * cellInv;
		float3 i = floor(big);
		float3 f = (big - i) + shiftM * cellInv + extra + 17.1 * k;
		i += floor(f);
		// As fbmTurned: the mean, 0.5, with no noise worked out where the pixel or the step is half a cell or more.
		float w = 1 - smoothstep(0.25, 0.5, foot);
		float octave = 0.5;
		[branch] if (w > 0)
			octave = lerp(0.5, noiseCell(i, frac(f)), w);
		sum += amp * octave;
		total += amp;
		cellInv *= 2.03;
		extra *= 2.03;
		foot *= 2.03;
		amp *= 0.5;
	}
	return sum / total;
}

// Elite's ground pass's cb2 as the frame's first ground draw had it (edcas.ini,
// [CommandListEdcasTerrainMask]); rows 13 and 14 are the camera's place, see camPlace. All zero
// where no ground was drawn.
cbuffer GroundCb : register(b12)
{
	float4 groundCb[16];
}

// The camera's place in the planet's frame, metres, as two parts a float holds: `anchorM`, a
// round number near it, and `fineM`, the rest.
//
// cb1[3] is that place in atmosphere radii, one float per axis: on a 3,578 km planet it moves in
// steps of 5 to 22 cm. A pattern placed by it stands still on the ground while the camera
// stands, and while the camera walks it is carried along and thrown back a step every few
// frames. The streamers are a metre apart and some centimetres wide: the whole sheet flickered
// at every step taken (the owner's clip of 2026-11-17, 15:01: nothing moves while standing; a
// replay of `ed_frame29156.rdc` with the camera one step on moved every streamer by its own
// width). Elite's ground pass has the place exactly, for its own cube-face textures: cb2[13], an
// anchor (multiples of 8192 m on every planet seen), and cb2[14], the rest, under 17 km, which a
// float holds to a millimetre; the camera is at minus their sum (the ground's point is its
// camera-relative position minus both). In two captures a second apart cb1[3] and the ground's
// own rounded cb2[12] were the same to the bit, and cb2[14] had moved 4.7 cm.
// From orbit the pair is in another frame (cb2[15] turns it), and with no ground drawn it is
// zero or an old one: unless it is within 2 m of cb1[3]'s place, that rounded place is taken,
// split the same way (a multiple of 8192 m and the rest).
void camPlace(Dust d, float3 cam, out float3 anchorM, out float3 fineM)
{
	float3 a = -groundCb[13].xyz, f = -groundCb[14].xyz;
	float3 off = (a + f) - cam * d.mPerUnit;
	bool exact = dot(off, off) < 4;
	float3 rounded = cam * d.mPerUnit;
	float3 own = round(rounded / 8192) * 8192;
	anchorM = exact ? a : own;
	fineM = exact ? f : rounded - own;
}

// fbmMoving at the point `camM + relM`: the camera's place in the planet's frame (metres) and the
// point's place from the camera (metres), kept apart. A point in the planet's frame is the camera's
// place (a number near 1 in atmosphere radii) plus the ray, and a float holds that sum to 0.2 to 0.4 m
// only: every pixel within that much of ground got the same point, and a pattern of 1.5 m cells was
// drawn in blocks that size (`ed_frame93219.rdc`, 2026-11-17: a grid over the sheet; streamers a
// sixth of a metre wide could not be drawn at all). The ray's own part is exact to millimetres, so
// it goes onto the place in the cell with the drift, where a float holds it finely. `camM` must
// not be the camera's rounded place: see camPlace.
float fbmMovingAt(float3 camM, float3 relM, float3 shiftM, float3 cellInv, float3 extra, const int n, float foot)
{
	float sum = 0, amp = 0.5, total = 0;
	[unroll] for (int k = 0; k < n; k++) {
		float3 big = camM * cellInv;
		float3 i = floor(big);
		float3 f = (big - i) + (relM + shiftM) * cellInv + extra + 17.1 * k;
		i += floor(f);
		float w = 1 - smoothstep(0.25, 0.5, foot);
		float octave = 0.5;
		[branch] if (w > 0)
			octave = lerp(0.5, noiseCell(i, frac(f)), w);
		sum += amp * octave;
		total += amp;
		cellInv *= 2.03;
		extra *= 2.03;
		foot *= 2.03;
		amp *= 0.5;
	}
	return sum / total;
}

// How far the wind has carried the dust's pattern, metres, at `speed`: downwind along the planet's
// fixed axis.
float3 driftM(Dust d, float speed)
{
	return -d.along * (speed * d.time);
}

// 0..1: how hard the wind gusts at `pM` (metres in the planet's frame) drifted by `shiftM`, seen
// through a pixel `footM` metres wide. Patches of about GUST_M that come and go over some tens of
// seconds (the time runs along the point's own up, which only shifts the pattern in place; the
// camera's would move it as it moves).
float gustAt(Dust d, float3 pM, float3 shiftM, float footM)
{
	return smoothstep(0.5, 0.78, fbmMoving(pM, shiftM, 1 / GUST_M, normalize(pM) * (d.time * 0.04), 3, footM / GUST_M));
}

// The cells a dust front is as large as, metres: at Jezero the daytime convection's cells are 2.4 to
// 5.3 km wide and a gust front lifted dust over at least 4 km2 (Newman et al. 2022). At 6 km and Mars's
// 16.6 m/s a cell passes in 6 minutes; the measured rhythm of gusts there is 9 to 15.
#define FRONT_M 6000.0

// 0..1: whether a dust front lies over `pM` (metres in the planet's frame). Where the wind only now
// and then gets over the threshold (a Mars-like body) the sand moves in bursts, seconds to minutes
// long, with clear air between: on Earth saltation was active 26 %, 6 % and 0.33 % of three hours as
// the wind fell, at Jezero dust was lifted for half a minute at a time (DUST-PLAN.md phase 5). A
// pattern of FRONT_M cells drifts with the gusts (at their speed: two speeds read as two winds,
// README-ELITE.md), and there is dust over the share d.activity of it (EDCAS: 0.19 on Mars, all of
// it from a gust 2.5 times the threshold on, as on 7 c). The noise's level over which that share of
// the ground lies is 0.5 + 0.11 ln((1 - a) / a): one octave of value noise has a mean of 0.5 and a
// deviation of 0.185. Measured in a replay over 4815 cells (2026-11-12): a front at half or more over
// 0.03, 0.20, 0.59 and 0.99 of them for 0.05, 0.19, 0.5 and 0.8; right where it counts, the thin
// worlds, and over towards the continuous end. At one place the share of the time differs by where it
// lies in the cells: 12 of 24 clock times a cell apart at ed_frame66376's camera.
float frontAt(Dust d, float3 pM)
{
	if (d.activity >= 1)
		return 1;
	float level = 0.5 + 0.11 * log((1 - d.activity) / d.activity);
	float n = fbmMoving(pM, driftM(d, 0.8 * d.wind), 1 / FRONT_M, 0, 1, 0);
	return smoothstep(level - 0.05, level + 0.05, n);
}

// The sheet's streamers. Blowing sand does not move as an even film: it runs in streamers, filaments
// some 0.2 m wide and about a metre apart that snake along the wind, intertwine and fork, come and
// go within seconds and leave the ground between them clear (Baas and Sherman 2005, on Earth;
// reports/Planetary dust and wind rendering.md: the saltation layer). Until client 0.43.204 the
// sheet was a mottled film (a noise of 1.5 m cells, 0.3 to 1.7 times, never empty), and it is lit
// as the ground under it averaged over 12 pixels: where it lay the picture was a soft copy of the
// ground, "as if one needed glasses" and no dust flowing (the owner, 2026-11-17;
// `ed_frame93219.rdc`).
//
// From client 0.43.204 to 0.43.209 a streamer was a line along a noise's contour, the noise's
// cells 30 times as long as wide: hard-edged ribbons of one width that ran straight for 30 m and
// more, cut off square where a second noise ended them, with nothing between. From eye height
// they were pale lanes fanning out from the wind's vanishing point, like tyre tracks or roads, and
// further off a comb of hairlines (the owner, 2026-11-17: "a very unnatural look";
// `ed_frame60273.rdc`, the dust alone: `ed-captures/dust-polish/`). Bends of 1 m every 8 m and 4 m
// every 30 m (client 0.43.207) did not change that: a contour of long cells is a rail.
//
// Now the streamers are lanes (`streamerLayer`): a streamer runs down each lane of a field that a
// noise pushes sideways by more than a lane's width every few metres, so the lanes snake, close up
// and open out; each lane's sand comes and goes along it by a noise of its own (soft, over metres,
// and all across the streamer alike: no cut ends), and across it the sand falls off as a bell
// (thick in the middle, no edge). Two layers of them, of different spacing and swing, lie over
// each other and cross: the intertwining and forking of the field reports.
// - STREAMER_M, STREAMER2_M: the two layers' lane spacing, metres. 1.3 and 2.9 m, the second with
//   STREAMER2_SHARE of the first's sand in each streamer: a streamer every 0.9 m across the wind.
// - STREAMER_WIDE_M: a streamer's width on Earth (at half its middle's sand), 0.2 m. A grain's hop
//   is as long as the gravity is low (0.3 m on Earth, 1 m on Mars: Kok 2010), and a filament can
//   not be narrower than its grains stray sideways while they hop, so the width goes with the
//   sheet's scale height, 0.1 m on Earth (loadDust): twice that, at least 0.2 m and at most
//   STREAMER_WIDEST_M, where the lanes begin to run into each other. An extrapolation: streamers
//   have been measured on Earth only (research_notes: 0.5 to 1 m on Mars "a reasonable guess").
// - STREAMER_SWING, _LONG_M, _WIDE_M (and 2): how many lanes a layer is pushed sideways, and the
//   cells of the noise that does it, along and across the wind. Across, the push changes by less
//   than a lane per lane, or the lanes would fold over into loops.
// - STREAMER_PIECE_M: the cells of a lane's own noise along it; a streamer is there for some
//   metres and gone.
// - STREAMER_FLOOR: between the streamers the sheet is this share of its mean, the grains that
//   creep.
// - FAMILY_M, FAMILY_LONG: the streamers come in families, bands some 12 m wide and three times as
//   long, which is what shows of them from further off than a streamer can be told; they sway
//   together (STREAMER_SWAY_M, STREAMER_SWAY: up to 3 m to either side every 30 m).
// The mean stays the sheet's (the same sand in all): where a streamer can not be told the sheet is
// as it was but for the families. It can not be told where a pixel is wider than the lanes, and
// where the view crosses the lanes low over the ground: the sheet is d.sheetM deep, and a ray
// that meets the ground at the angle a runs sheetM / tan a along the ground inside it; looking
// across the wind from eye height that is more than a lane from a few metres on, and the
// streamers one behind the other close up into a haze, as blowing sand seen from the side does,
// while down the wind they stay apart to where the pixels end them.
#define STREAMER_M 1.3
#define STREAMER2_M 2.9
#define STREAMER2_SHARE 0.6
#define STREAMER_WIDE_M 0.2
#define STREAMER_WIDEST_M 0.6
#define STREAMER_SWING 1.2
#define STREAMER_SWING_LONG_M 9.0
#define STREAMER_SWING_WIDE_M 3.0
#define STREAMER2_SWING 0.6
#define STREAMER2_SWING_LONG_M 11.0
#define STREAMER2_SWING_WIDE_M 5.0
#define STREAMER_PIECE_M 7.0
#define STREAMER_SWAY_M 30.0
#define STREAMER_SWAY 3.0
#define STREAMER_FLOOR 0.1
#define FAMILY_M 12.0
#define FAMILY_LONG 3.0
// The wind's eddies. Carried along one axis as one stiff pattern (to client 0.43.210), every
// streamer moved in exactly the same direction at exactly the same speed, which no wind does (the
// owner, 2026-11-17: "all the wind streaks are moving in the exact same direction, which looks
// unnatural"; measured on a sketch of the pattern, 3 m blocks matched between two frames: every
// block's motion within 3 degrees of the axis, the measurement's own noise). Near the ground the
// wind is turbulent: over flat land its speed across the mean wind has a standard deviation of
// 1.9 times the friction velocity and along it of 2.4 times (Panofsky and Dutton 1984, as cited by
// later boundary-layer papers), and the mean wind a metre up is 13 to 17 times that velocity (the
// log law over ground a millimetre rough), so the wind there veers by 6 to 8 degrees (one
// deviation) and its speed changes by 14 to 18 %, in eddies some tens of metres long that the wind
// carries along.
// - EDDY_LONG_M, EDDY_WIDE_M: the eddies' cells along and across the wind, metres.
// - EDDY_ACROSS, EDDY_ALONG: the eddy's wind across and along the mean wind at the noise's ends,
//   as shares of the wind's speed: 0.33 and 0.43, which is a deviation of 0.12 (7 degrees) and
//   0.16 (one octave of value noise has a deviation of 0.185, twice that from -1 to 1).
// - EDDY_TURN_S: a pattern can not be carried by a wind that differs from place to place for long
//   (it shears without end), so there are two, each carried by the local wind for this many
//   seconds from a new place and faded in and out over it, half a turn apart (a flow map): a
//   streamer lives that long, as they "come and go within seconds".
// With the eddies alone (client 0.43.211) the streamers still moved "too steady", "not dynamic
// enough" (the owner, 2026-11-17): a conveyor belt. Three things were even that a wind's are not:
// - The sand. A faster stretch of wind carried no more sand than a slow one. The sand a wind moves
//   goes with its stress over the threshold's, u*^2 - u*t^2 (Martin and Kok 2017), and follows the
//   wind within about a second (Kok et al. 2012): with the eddy's wind along the mean wind, e
//   (EDDY_ALONG times -1..1), it is ((1 + e)^2 - q) over that's mean, q the threshold's stress over
//   the gust's: 1 / S^2 with EDCAS's gust ratio S, which the shader has as the fronts' share
//   (planet_dust.rs: activity = (S - 0.3) / 2.2), held between 1 (the sand moves in the surges
//   only) and 2.5 (the ratio from which EDCAS writes a share of 1, whatever it is beyond). The mean
//   over the eddies' noise is 1.0253 - q + 0.47 max(q - 0.5, 0)^2 (worked out on two million
//   samples of it for EDDY_ALONG 0.43, to 0.002). At S = 2.5 that is 0.63 to 1.37 times the sand
//   at one deviation of the wind, 0.19 to 2.2 at the noise's ends: bursts with thinner stretches
//   between, which pass in three to four seconds (the eddies' cells in the wind).
// - The streamers' shapes. The noise that pushes the lanes sideways changed in ten seconds, a
//   streamer lives four: each slid along as it was made. An eddy turns over in its size over its
//   own wind: the push's cells are STREAMER_SWING_WIDE_M across, the wind across the mean wind
//   has a deviation of 0.12 to 0.16 of it, so the noise now changes EDDY_WRITHE times the streamers'
//   speed over that width a second (0.47 a second at 10 m/s: a streamer bends and
//   straightens within its life).
// - The turns. Every place turned its two patterns over once in EDDY_TURN_S, only at its own
//   time. A slow noise (EDDY_PACE_LONG_M by EDDY_PACE_WIDE_M cells, carried by the wind, EDDY_PACE
//   turns from end to end, changing over some ten seconds) now sets the time, so a place turns
//   faster while the noise rises and slower while it falls: streamers live three to seven seconds.
//   (The eddy's own noise set the time before: finer cells, which sheared the pattern.)
#define EDDY_LONG_M 30.0
#define EDDY_WIDE_M 12.0
#define EDDY_ACROSS 0.33
#define EDDY_ALONG 0.43
#define EDDY_TURN_S 4.0
#define EDDY_WRITHE 0.14
#define EDDY_PACE_LONG_M 60.0
#define EDDY_PACE_WIDE_M 40.0
#define EDDY_PACE 3.0

// Value noise along a lane and in time, each lane its own: at the cell `cell` and the place
// `place` (0..1) in it along the lane, the lane's number `lane`, the time `t` in cells.
float laneNoise(float cell, float place, float lane, float t)
{
	float tick = floor(t), part = t - tick;
	place = place * place * (3.0 - 2.0 * place);
	part = part * part * (3.0 - 2.0 * part);
	return lerp(lerp(hashU(float3(cell, lane, tick)), hashU(float3(cell + 1, lane, tick)), place),
	            lerp(hashU(float3(cell, lane, tick + 1)), hashU(float3(cell + 1, lane, tick + 1)), place), part);
}

// One layer of streamers at the point `camM + relM` (as fbmMovingAt), carried `drift` by the wind:
// how many times the layer's mean sand lies there. Lanes `spacing` metres apart across the wind
// (`across`; `along` the wind, `high` the third axis, the noises' time), pushed sideways `swing`
// lanes to either side by a noise of `swingLongM` by `swingWideM` cells; a streamer's sand falls
// off across it as exp(-(x / sigmaM)^2), and comes and goes along it by the lane's own noise
// (laneNoise, `seed` apart from the other layer's). `footM`: the pixel's width on the ground.
// The streamer of the lane the point is in and the next lane's on its side are summed, so the
// sand is whole across a lane's edge, where a wide streamer still has some. Wider than half a
// lane the two no longer make up the row (the lanes beyond are missing), and there is little left
// to tell: the mean.
float streamerLayer(Dust d, float3 camM, float3 relM, float3 drift, float3 along, float3 across, float3 high,
                    float spacing, float sigmaM, float smearM, float swing, float swingLongM, float swingWideM, float seed, float footM)
{
	// The bell's width in lanes. The ray's run inside the sheet thins out upwards as the sheet does, which
	// takes a row of streamers `spacing` apart down to 1 / sqrt(1 + (2 pi run / spacing)^2) of its contrast:
	// slowly, a run of four lanes still leaves a twelfth. As the widening of the bell that does the same.
	float run = 6.2832 * smearM / spacing;
	float sigma = sqrt(sigmaM * sigmaM / (spacing * spacing) + log(1 + run * run) / 19.74);
	float told = 1 - smoothstep(0.45, 0.65, sigma);
	float sand = 1;
	[branch] if (told > 0) {
		// The push turns over as an eddy its width across does (EDDY_WRITHE).
		float push = fbmMovingAt(camM, relM, drift, along / swingLongM + across / swingWideM, high * (d.time * EDDY_WRITHE * d.flow / swingWideM), 2, footM / swingWideM) - 0.5;
		// The lane, as fbmMovingAt's cells: the camera's round part gives whole lanes and a rest, the ray goes onto the rest.
		float big = dot(across, camM) / spacing;
		float whole = floor(big);
		float f = (big - whole) + dot(across, relM) / spacing + 2 * swing * push;
		float lane = whole + floor(f);
		float x = frac(f) - 0.5;
		float bigAlong = dot(along, camM) / STREAMER_PIECE_M;
		float cell = floor(bigAlong);
		float place = (bigAlong - cell) + dot(along, relM + drift) / STREAMER_PIECE_M;
		cell += floor(place);
		place = frac(place);
		float t = d.time * 0.3 + seed;
		float own = smoothstep(0.3, 0.7, laneNoise(cell, place, lane, t));
		float next = smoothstep(0.3, 0.7, laneNoise(cell, place, lane + (x >= 0 ? 1 : -1), t));
		float fromOwn = abs(x) / sigma, fromNext = (1 - abs(x)) / sigma;
		// Over the mean of such a row of bells, half of them there: sqrt(pi) / 2 * sigma.
		sand = lerp(1, (own * exp(-fromOwn * fromOwn) + next * exp(-fromNext * fromNext)) / (0.886 * sigma), told);
	}
	return sand;
}

// How many times the sheet's mean the sand is at the point `relM`: metres east (x) and north (y)
// of the round part of the camera's place (sheetDepth lays the frame out), z nothing; seen through
// a pixel `footM` metres wide along the ray on the ground and `footAcrossM` across the wind, the
// ray running `smearM` across the wind inside the sheet: the streamers and their families, moving
// east with the wind and its eddies and changing as they go. The noises' third axis is their time.
// The frame is a flat one about a point at most 17 km from the camera (Elite's anchor, camPlace):
// east and north are the camera's, so the wind is the same all over the picture and turns with the
// planet as the camera travels; the pattern then slides by the turn times the distance to that
// point, under 0.5 % of the camera's own way. Where Elite takes a new anchor (every 8 km or more)
// the streamers are others from one frame to the next.
// Laid on two of the planet's own axes instead (client 0.43.205 to 0.43.212: the two the ground lies
// along most) the pattern stood still on the planet, but the wind could only blow along an axis,
// and along the edges of a cube round the planet there was a line the streamers did not cross. On
// all three axes with long cells along one of them (client 0.43.204) the lines closed into rings
// where the ground is tilted against that axis (the owner's clip of 2026-11-17, at 2.9 s).
float streamers(Dust d, float3 relM, float footM, float footAcrossM, float smearM)
{
	float3 along = float3(1, 0, 0), across = float3(0, 1, 0), high = float3(0, 0, 1);
	float3 camM = 0;
	float3 drift = -along * (d.flow * d.time);
	// The families and the streamers sway together.
	float sway = fbmMovingAt(camM, relM, drift, along / STREAMER_SWAY_M + across / (2 * FAMILY_M), high * (d.time * 0.03), 2, footM / STREAMER_SWAY_M) - 0.5;
	float3 swayed = relM + across * (2 * STREAMER_SWAY * sway);
	float family = fbmMovingAt(camM, swayed, drift, along / (FAMILY_M * FAMILY_LONG) + across / FAMILY_M, high * (d.time * 0.05), 2, footM / FAMILY_M);
	// Thicker and thinner by the families, never bare between them (a bare stripe beside a full one read as rays).
	float band = 0.4 + 1.2 * smoothstep(0.3, 0.7, family);
	// A streamer's bell: its own width and the pixel's, across the wind.
	float wide = clamp(2 * d.sheetM, STREAMER_WIDE_M, STREAMER_WIDEST_M) / 1.665;
	float sigmaM = sqrt(wide * wide + 0.36 * footAcrossM * footAcrossM);
	// Each layer by its share of the sand: streamers to the metre, and the second's are thinner.
	float share = (STREAMER2_SHARE / STREAMER2_M) / (1 / STREAMER_M + STREAMER2_SHARE / STREAMER2_M);
	// The eddies (EDDY_*): the wind's own, across and along, where the point is; the wind carries them.
	float3 cells = along / EDDY_LONG_M + across / EDDY_WIDE_M;
	float veer = fbmMovingAt(camM, relM, drift, cells, high * (d.time * 0.05), 1, footM / EDDY_WIDE_M);
	float surge = fbmMovingAt(camM, relM, drift, cells, high * (d.time * 0.05) + 51.3, 1, footM / EDDY_WIDE_M);
	float3 eddy = d.flow * (across * (EDDY_ACROSS * (2 * veer - 1)) + along * (EDDY_ALONG * (2 * surge - 1)));
	// Two patterns of streamers, each carried by the eddy for EDDY_TURN_S from a place of its own (by the
	// turn's number, so no turn repeats the one before) and faded in and out over it; each place turns
	// them over at its own time and pace (`pace`: all at once, the whole sheet would beat). The weights add up
	// to 1, so the sand stays the sheet's all through a turn. Two patterns half there spread it more evenly
	// than one whole, and the dust's veil is not as thick again where the sand is (1 - exp(-depth)): the same
	// sand shows a tenth more in the picture (`ed_frame60273.rdc`: 2.59 of 255 in the green, 2.35 before).
	float pace = fbmMovingAt(camM, relM, drift, along / EDDY_PACE_LONG_M + across / EDDY_PACE_WIDE_M, high * (d.time * 0.08) + 23.7, 1, footM / EDDY_PACE_WIDE_M);
	float sand = 0;
	[unroll] for (int k = 0; k < 2; k++) {
		float phase = d.time / EDDY_TURN_S + 0.5 * k + EDDY_PACE * pace;
		float turn = floor(phase);
		float age = (phase - turn - 0.5) * EDDY_TURN_S;
		float weight = 1 - abs(age) / (0.5 * EDDY_TURN_S);
		float place = turn - 64 * floor(turn / 64) + 100 * k;
		float3 moved = swayed - eddy * age + along * (37.7 * place) + across * (11.3 * place);
		float first = streamerLayer(d, camM, moved, drift, along, across, high, STREAMER_M, sigmaM, smearM, STREAMER_SWING, STREAMER_SWING_LONG_M, STREAMER_SWING_WIDE_M, 0, footM);
		float second = streamerLayer(d, camM, moved, drift, along, across, high, STREAMER2_M, sigmaM, smearM, STREAMER2_SWING, STREAMER2_SWING_LONG_M, STREAMER2_SWING_WIDE_M, 100, footM);
		sand += weight * lerp(first, second, share);
	}
	// The sand the eddy's wind moves, over the mean wind's (EDDY_*: the sand); the mean's where a pixel
	// is too wide to tell the eddies.
	float gustRatio = clamp(0.3 + 2.2 * d.activity, 1, 2.5);
	float q = 1 / (gustRatio * gustRatio);
	float faster = 1 + EDDY_ALONG * (2 * surge - 1);
	float tight = max(q - 0.5, 0);
	float burst = lerp(1, max(faster * faster - q, 0) / (1.0253 - q + 0.47 * tight * tight), 1 - smoothstep(0.25, 0.5, footM / EDDY_WIDE_M));
	return burst * band * (STREAMER_FLOOR + (1 - STREAMER_FLOOR) * sand);
}

// The saltation sheet over the ground where a ray ends: its optical depth, from the camera's
// height down to the ground along the ray, in streamers along the wind (`streamers`) and stronger in gusts.
// `pixelAngle`: the angle one pixel spans (radians), `normal` the terrain's, `front` frontAt there.
float sheetDepth(Dust d, float3 cam, float3 dir, float ground, float pixelAngle, float3 normal, float front)
{
	float3 hit = cam + dir * ground;
	float slope = groundSlope(dir, hit, normal);
	float path = sheetPath(d, dir, hit, ground, d.sheetM, slope);
	// The ground one pixel covers, in metres: stretched along the ray where it meets the
	// ground at a flat angle.
	float footM = ground * d.mPerUnit * pixelAngle / slope;
	float3 pM = hit * d.mPerUnit;
	float gust = gustAt(d, pM, driftM(d, 0.8 * d.wind), footM) * front;
	// Gone towards the horizon by the angle the ray meets the planet at, as well as by the
	// distance: on a plain seen from eye height the ground from DUST_NEAR_M to DUST_FAR_M lies
	// in a few pixels' rows just under the horizon, and the fade by distance alone was a hard
	// line there, at eye level, that moved as the commander crouched (2026-10-29). Over 0.2 to
	// 1.1 degrees it spans some tens of rows; over 0.3 to 3 it took the sheet from beyond 35 m on
	// foot and the dust was hardly seen (2026-10-29).
	float near = (1 - smoothstep(DUST_NEAR_M, DUST_FAR_M, ground * d.mPerUnit)) * smoothstep(0.003, 0.02, -dot(dir, normalize(hit)));
	// 0.12 / m was tuned for a sheet of SHEET_M: a shallower one is as much denser, so that looking
	// straight down the sheet is as thick as it was.
	// The sheet's floor between the gusts, 0.15, by the share of fronts: between them the ground is
	// clear on a Mars-like body, and where the sand always moves (activity 1) it is as it was.
	// The streamers are placed from the camera's exact place (camPlace), the rest of it with the ray.
	float3 anchorM, fineM;
	camPlace(d, cam, anchorM, fineM);
	float3 placeM = fineM + dir * (ground * d.mPerUnit);
	// The streamers' frame: east, the wind's way, and north, across it, at the camera.
	float3 north = cross(d.up, d.east);
	// Across the wind: the pixel's width on the ground, long along the ray and short across it, and the
	// ray's run along the ground inside the sheet.
	float3 flat = dir - normalize(hit) * dot(dir, normalize(hit));
	float crossing = abs(dot(flat, north)) / max(length(flat), 1e-6);
	float shortM = ground * d.mPerUnit * pixelAngle;
	float footAcrossM = sqrt(lerp(shortM * shortM, footM * footM, crossing * crossing));
	float smearM = d.sheetM * sqrt(max(1 - slope * slope, 0)) / slope * crossing;
	return 0.12 * (SHEET_M / d.sheetM) * near * path * (0.15 * d.activity + gust) * streamers(d, float3(dot(d.east, placeM), dot(north, placeM), 0), footM, footAcrossM, smearM);
}

// The billows' density at a point (1 / m), lifted in the gusts. `stepM`: the march's step there
// (metres): puffs and gusts finer than the steps can resolve are averaged out, or each pixel's
// jittered steps would land on different ones and the air would sparkle (2026-10-29).
float billowAt(Dust d, float3 p, float h, float stepM, float front)
{
	if (h > 4 * BILLOW_M)
		return 0;
	float3 pM = p * d.mPerUnit, shiftM = driftM(d, 0.8 * d.wind);
	float gust = gustAt(d, pM, shiftM, stepM) * front;
	if (gust <= 0)
		return 0;
	float puff = fbmMoving(pM, shiftM, 1.0 / 7, normalize(pM) * (d.time * 0.15), 3, stepM / 7);
	return 0.06 * gust * gust * exp(-h / BILLOW_M) * smoothstep(0.35, 0.7, puff);
}

// The Henyey-Greenstein phase function with asymmetry g, 1 on average over the sphere.
float henyeyGreenstein(float g, float cosine)
{
	return (1 - g * g) / pow(max(1 + g * g - 2 * g * cosine, 1e-3), 1.5);
}

// How much of the land's relief between a view ray and the ground it is measured from is taken on
// trust, metres. The billows and the far drift lie over the ground, and the ground under a point of
// a ray is not known: only the lowest ground seen 300 m to 3 km from the camera (edcas_ground_ref.hlsl)
// and the ground where the ray ends. A point under those counts as at ground level (a hollow the
// camera stands in, a slope the ray climbs along) as far as this under the lowest ground and this
// far from the terrain's own plane at the ray's end; deeper or further it is not near the ground,
// and the layers thin out as they do above it. Without a limit all the air under that level was
// full of dust: from a ship 1.65 km over a crater's floor, facing its wall, no ground lay 300 m to
// 3 km off in view, the probe had no level at all (its weight 0: read out 2026-11-12) and the level
// was the planet's own radius, over 3 km above the ship; the drift lay at full density along every
// ray at eye level and over, a grey band on the wall with a hard line at eye level and a haze on the sky
// (Slege ZL-I b43-1 2, 2026-11-11, `ed-captures/renderdoc/ed_frame171605.rdc`). At 0 from the
// terrain's plane the dust on rising ground 100 to 400 m off, as tuned in game, lost two fifths
// (`ed_frame66376.rdc`).
#define HOLLOW_M 30.0

// The height (metres) the billows and the far drift take a point of a view ray at. `rP`: the point's
// distance from the planet's centre; `rFloor`: the level of the lowest ground in view; `onGround`:
// the ray ends on the ground (not in the sky, on a hull or a plant: the lowest ground alone counts
// then), at `rGround` from the centre; `toEndM`: metres left to the ray's end; `facing`: the sine of
// the angle the ray meets the terrain at there (terrainFacing); `hollow`: how far from the terrain's
// plane there a point still counts as on the ground, metres; `end`: 0 along the ray, 1 in its last
// fifth.
// Along the ray the height is taken over the lowest ground in view or over the ray's end, whichever
// is higher, and in the last fifth over the ray's end alone: taken over a far hill's ground all the
// way, the air in front of it was at ground level for the whole 400 m and the hill turned white
// (2026-10-29). Over the ray's end it is the point's height over that ground's radius, or its
// distance from the terrain's own plane there less `hollow`, whichever is more: a ray that climbs to
// its end (a crater's wall, a hill over eye level) runs under that radius all the way.
// `hollow` is HOLLOW_M for a camera inside the billows and for the far drift, and nothing for the
// billows seen from over them (groundDust's `above`): a ray that comes in low over a slope facing
// the camera runs near the radius of its end for hundreds of metres, and with 30 m of the air in
// front of the slope counted as ground level it crossed 36 m / facing of dust at full density where
// the layer itself is 6 m / facing deep: a veil without a shape over every hillside, 13 times as
// bright as the same land seen from inside the billows, which came and went as the camera crossed
// the 24 to 54 m over the ground between the two (the owner, 2026-11-13: "at a certain angle,
// suddenly the ground fog appears way brighter", `ed_frame53857.rdc`, the outside camera 58 m over a
// hillside: the dust added 15.2 levels to the land it lies on, 5.4 without the 30 m, 1.2 from
// inside). Looking down from a ship the radius decides and nothing changes (`ed_frame169294.rdc`:
// 0.04 % of the picture by more than 4 levels).
float dustHeight(float rP, float rFloor, bool onGround, float rGround, float toEndM, float facing, float hollow, float end, float mPerUnit)
{
	float overFloor = (rP - rFloor) * mPerUnit;
	overFloor = overFloor >= 0 ? overFloor : max(-overFloor - HOLLOW_M, 0);
	if (!onGround)
		return overFloor;
	float overEnd = max((rP - rGround) * mPerUnit, toEndM * facing - hollow);
	return max(lerp(max(overFloor, overEnd), overEnd, end), 0);
}

// The picture's light on the ground round a pixel that shows something else (groundDust): the sum
// over those of 12 pixels in a spiral `radius` pixels wide (a Vogel disc) that show the ground, by
// the terrain mask as the pixel's own test, and in w how many do. `slope`: the pixel's ray over its
// view depth, across and up the view; it changes by `slopeX` and `slopeY` a pixel all over the
// picture, so a tap's distance along its own ray is exact however far off it is (the first-order
// step the ground's own 12 pixel disc takes is 2 % out at 190 pixels near the picture's edge, the
// mask test's whole margin).
// The sum of the picture's light over a disc's taps (`taps` of the 12 show the ground; the others are 0), the
// brightest quarter of them held down to the next one's brightness. One pixel of the 12 that shows a glint of
// the ice (EDCAS's sparkle, or Elite's own glitter: a facet catching the sun, tens to hundreds of times the
// ground round it) lit its texel's dust up, and so did every texel whose spiral met that pixel: each glint
// stood in a cluster of white blobs, and where the glints are thick, looking low towards the sun, the dust
// over the ice was strewn with them (the owner, 2026-11-13, `ed_frame41966.rdc`: the sparkle changed 7 722
// pixels of the picture without the dust and 35 836 with it, in blobs up to 4 984 pixels). The dust is lit by
// the light that falls on the ground, not by what single facets mirror into the camera. Held down, not left
// out: the mean of even ground stays what it was, and at a shadow's edge the sunlit half still counts in full.
float3 calmSum(float3 tap[12], float taps)
{
	float3 weights = float3(0.2126, 0.7152, 0.0722);
	float lum[12];
	int k;
	[unroll] for (k = 0; k < 12; k++)
		lum[k] = dot(tap[k], weights);
	// The brightness of the tap after the brightest quarter (the fourth of 12): each turn finds the brightest
	// one under the last.
	float limit = 1e30;
	[unroll] for (int turn = 0; turn < 4; turn++) {
		float next = 0;
		[unroll] for (k = 0; k < 12; k++)
			next = max(next, lum[k] < limit ? lum[k] : 0);
		limit = turn * 4 <= taps && next > 0 ? next : limit;
	}
	float3 sum = 0;
	[unroll] for (k = 0; k < 12; k++)
		sum += tap[k] * (lum[k] > limit ? limit / lum[k] : 1);
	return sum;
}

float4 groundLightDisc(int2 pixel, float radius, bool alt, float metresPerDepth, float2 slope, float2 slopeX, float2 slopeY)
{
	float3 found[12];
	float taps = 0;
	float along = sqrt(1 + dot(slope, slope));
	[unroll] for (int k = 0; k < 12; k++) {
		float angle = k * 2.39996, r = radius * sqrt((k + 0.5) / 12);
		int2 offset = int2(round(r * float2(cos(angle), sin(angle))));
		int3 tap = int3(pixel + offset, 0);
		float tapDepth = alt ? DepthAlt.Load(tap).x : Depth.Load(tap).x;
		float2 s = slope + slopeX * offset.x + slopeY * offset.y;
		float tapM = tapDepth * metresPerDepth * sqrt(1 + dot(s, s)) / along;
		float tapMaskM = TerrainDist.Load(tap);
		bool on = tapMaskM > 0 && abs(tapMaskM - tapM) < 0.2 + 0.02 * tapM;
		found[k] = on ? Scene.Load(tap).rgb : 0;
		taps += on;
	}
	return float4(calmSum(found, taps), taps);
}

// The sine of the angle a view ray meets the terrain at, with the terrain's plane taken over the
// billows' own depth: from the ground 2 * BILLOW_M to each side of the ray's end, across and up the
// picture (at least 2 pixels, at most 48). `fallback`: the same from the pixel's own neighbours
// (groundDust's `normal`), which follows every facet of the terrain: a debug picture of it on a
// slope 1 km off was rows of dashes 30 pixels long and 8 apart, and seen from above, where the
// billows' stretch of the ray is as long as 1 / this, the dust was drawn in those dashes, bright and
// dark: "pixels going up the hill" as the wind carried the gusts through them (the owner,
// 2026-11-13, `ed_frame53857.rdc`: with this fixed at 0.3 they were gone, with the gusts' and the
// puffs' noise changed in six ways they stayed). The billows lie metres deep over the ground and
// do not follow its facets.
// A plane's 1 / view depth changes by the same amount from pixel to pixel all over the picture.
// Where the two sides of the ray's end do so, the plane runs through both; where they do not (a
// ridge with other ground behind it) it is taken from the side that changes less; a side that is
// not ground by the terrain mask (a ship, a rock, the sky, the picture's edge) is left out.
// terrainPlane: that plane's normal (not of length 1) in the view's frame, across, up and into the
// picture, from the ground `reach` pixels to each side; false where there is none to be had.
// `strict`: only where both sides of the ray's end lie in one plane with it, across and up the
// picture: the ray's end no further from the line through them than a twentieth of their
// distance (a bend tighter than a radius of 2.5 times that distance is no plane; from the line,
// not from the middle between them: seen flat, the near side is much nearer than the far one is
// far). Not by the test
// of the 1 / view depth below: where the ground lies level across the picture that does not change
// from side to side at all, the test's two differences are both next to nothing, and it fails over
// whole stretches of even ground (a debug picture of `ed_frame60273.rdc`: most of the plain). And
// a side left out is no plane to go by: a slope's tilt is wanted here, not a guess at it.
bool terrainPlane(int2 pixel, bool alt, float groundM, float metresPerDepth, float2 slope, float2 slopeX, float2 slopeY, int2 reach, bool strict, out float3 plane)
{
	plane = 0;
	float along = sqrt(1 + dot(slope, slope));
	float3 centre = float3(slope, 1) * (groundM / along);
	float w0 = along / max(groundM, 1e-6);
	float3 span[2];
	[unroll] for (int axis = 0; axis < 2; axis++) {
		float3 side[2];
		float w[2];
		bool on[2];
		[unroll] for (int k = 0; k < 2; k++) {
			int2 offset = (axis == 0 ? int2(reach.x, 0) : int2(0, reach.y)) * (2 * k - 1);
			int3 tap = int3(pixel + offset, 0);
			float2 s = slope + slopeX * offset.x + slopeY * offset.y;
			float z = (alt ? DepthAlt.Load(tap).x : Depth.Load(tap).x) * metresPerDepth / along;
			float tapM = z * sqrt(1 + dot(s, s));
			float tapMaskM = TerrainDist.Load(tap);
			on[k] = tapMaskM > 0 && abs(tapMaskM - tapM) < 0.2 + 0.02 * tapM;
			side[k] = float3(s, 1) * z;
			w[k] = 1 / max(z, 1e-6);
		}
		float before = abs(w0 - w[0]), after = abs(w[1] - w0);
		if (strict) {
			float3 chord = side[1] - side[0];
			if (!(on[0] && on[1] && length(cross(centre - side[0], chord)) <= 0.05 * dot(chord, chord)))
				return false;
			span[axis] = side[1] - side[0];
		} else if (on[0] && on[1] && abs((w0 - w[0]) - (w[1] - w0)) <= 0.25 * (before + after) + 1e-4 * w0)
			span[axis] = side[1] - side[0];
		else if (on[1] && (!on[0] || after < before))
			span[axis] = side[1] - centre;
		else if (on[0])
			span[axis] = centre - side[0];
		else
			return false;
	}
	plane = cross(span[0], span[1]);
	return dot(plane, plane) > 0;
}

float terrainFacing(int2 pixel, bool alt, float groundM, float metresPerDepth, float2 slope, float2 slopeX, float2 slopeY, float metresPerPixel, float fallback)
{
	int reach = (int)clamp(2 * BILLOW_M / max(metresPerPixel, 1e-6), 2, 48);
	float3 plane;
	if (!terrainPlane(pixel, alt, groundM, metresPerDepth, slope, slopeX, slopeY, int2(reach, reach), false, plane))
		return fallback;
	return abs(dot(plane, float3(slope, 1))) / (length(plane) * sqrt(1 + dot(slope, slope)));
}

// How much of the sheet's sand moves on the ground at a pixel, by how that ground lies to the wind:
// none on a slope that falls away down the wind more steeply than the flow can follow. Over a
// smooth hill the wind stays on the ground as long as its lee slope is under about 0.3 (17
// degrees; Wood 1995, the onset of separation in neutral flow over hills: the critical slope goes
// with the hill's length over the ground's roughness, 0.3 to 0.35 for land like this); steeper,
// it leaves the ground at the brink and the air under it is slack or runs back, so the sand that
// comes over drops there: that is how a dune's slip face, 32 to 34 degrees, is made. The sheet
// thins from a fall of LEE_FROM along the wind (17 degrees) and is gone at LEE_TO (30 degrees).
// From 10 to 20 degrees, as first written (the 14 degrees of the dune models, which is how
// steeply the streamline falls behind a sharp brink, not where the flow leaves a slope), a whole
// mountainside that falls 16 degrees to the east lost its dust (`ed_frame53857.rdc`): down a long
// even slope the wind just follows the ground.
// The slope is the terrain's plane over LEE_BASE_M to each side on the ground (terrainPlane), some
// hops of a grain: the sand hops over what is smaller, and the pixel's own facets (groundDust's
// `normal`) would strew the sheet with holes. That is LEE_BASE_M over a pixel's width across the
// picture and, up it, as much less as the ray meets the ground flat (`grazing`, its sine): as many
// pixels up as across reach tens of metres of ground from eye height, over which no land is a plane.
// Near the camera, where it is more than LEE_REACH pixels, it is as far as those reach.
// And no further than the picture goes (`room`: the pixels to its nearest edge, across and up):
// with the whole reach, a frame as wide as it round the picture had no plane, and so its dust on
// every lee slope.
// (A debug picture of the fall on `ed_frame60273.rdc` showed dotted winding lines of steep lee and
// windward ground all over an even plain, which no change of the reach moved: not the land, but
// the depth pixel a texel takes, see viewRay.)
// What this pass can not know is left out: the slack air goes on behind the slope's foot (4 to 10
// times its height), and the wind is stronger over a crest and weaker in a hollow, by far more than
// by the slope it blows up (the same models: the part that goes with the land's shape as a whole).
// Both need the land's height upwind, which is not in the picture where it lies behind the camera;
// taken from the picture the dust would come and go as the camera turns.
#define LEE_BASE_M 3.0
#define LEE_REACH 160
#define LEE_FROM 0.31
#define LEE_TO 0.58
float leeShelter(Dust d, int2 pixel, bool alt, float groundM, float metresPerDepth, float2 slope, float2 slopeX, float2 slopeY, float metresPerPixel, float grazing, int2 room)
{
	float across = LEE_BASE_M / max(metresPerPixel, 1e-6);
	int2 reach = int2(max(min(clamp(float2(across, across * grazing), 1, LEE_REACH), float2(room)), 1));
	float3 plane;
	if (!terrainPlane(pixel, alt, groundM, metresPerDepth, slope, slopeX, slopeY, reach, true, plane))
		return 1;
	// Into the planet's frame, as a view ray is (viewRay), and facing up.
	float3 n = plane.x * cb1[277].xyz + plane.y * cb1[278].xyz + plane.z * cb1[279].xyz;
	n = float3(dot(cb1[19].xyz, n), dot(cb1[20].xyz, n), dot(cb1[21].xyz, n));
	float upward = dot(n, d.up);
	// The fall along the wind, metres a metre: a surface that rises h' along the wind has its normal along (-h', 1).
	float fall = dot(n, d.east) * sign(upward) / max(abs(upward), 1e-6 * length(n));
	return 1 - smoothstep(LEE_FROM, LEE_TO, fall);
}

// How far a disc that found `taps` of its 12 pixels on the ground is trusted, 0 to 1: the share of
// the dust's light taken from it. What it leaves goes to the next wider disc, and what the widest
// leaves to the open sun (groundDust). In full from 4 pixels on (the owner's choice of three,
// 2026-11-12, ed-captures/dust-patches/trust/): from 1 on is the same picture but for a few hundred
// pixels, and takes the light at a rock's edge from a single pixel, which may be a ripple's
// highlight; taps / 12 hands the sun the share of every disc half on the ground, and 1 600 pixels
// of the patches stayed, up to 124 levels.
float discTrust(float taps)
{
	return saturate(taps / 4.0);
}

// The dust along a pixel's view ray, premultiplied: in front of the fog and the clouds.
float4 groundDust(float3 v0, float3 v1, Ray ray)
{
	Dust d;
	if (!loadDust(d))
		return 0;
	float3 dir = ray.dir;
	int2 pixel = ray.pixel;
	float ground = ray.ground;
	// Taken here, before any branch that differs between pixels: the angle a pixel spans and
	// the terrain's normal, facing the camera, from the hit relative to the camera (precise,
	// where the planet-centred hit is only good to a tenth of a metre).
	float pixelAngle = max(length(ddx(dir)), length(ddy(dir)));
	// For the picture's pixels round this one (the dust's light, below): the length along a ray
	// per metre of view depth, how it changes from texel to texel, and the picture's pixels a texel.
	float alongPerDepth = length(v1) / dot(v1, cb1[279].xyz);
	float2 alongStep = float2(ddx(alongPerDepth), ddy(alongPerDepth));
	// The depth's pixels a texel of this pass, from the picture's place itself: taken as the step from
	// one texel's depth pixel to the next (a whole number cut off) it was right only where the depth is
	// a whole number of times this pass's size, and even there not everywhere (viewRay: the pixel).
	// With the picture rendered at another scale (`ed_frame53857.rdc`) the step goes 1, 2, 1, 2 in a
	// grid, every pixel read round this one was looked for along the wrong ray, and a plane laid
	// through four of them (terrainPlane) was tilted all over the land.
	bool alt = asint(cb1[25].w) == 1;
	uint depthWidth, depthHeight;
	if (alt)
		DepthAlt.GetDimensions(depthWidth, depthHeight);
	else
		Depth.GetDimensions(depthWidth, depthHeight);
	float2 place = v0.xy / v0.z * float2(0.5, -0.5) + 0.5;
	float2 pixelsPerTexel = max(abs(float2(ddx(place.x), ddy(place.y))) * float2(depthWidth, depthHeight), 0.25);
	// And for pixels any distance off (groundLightDisc): the ray over its view depth, across and up
	// the view. It changes by the same amount from pixel to pixel all over the picture.
	float2 slope = float2(dot(v1, cb1[277].xyz), dot(v1, cb1[278].xyz)) / dot(v1, cb1[279].xyz);
	float2 slopeX = ddx(slope) / pixelsPerTexel.x, slopeY = ddy(slope) / pixelsPerTexel.y;
	float3 rel = dir * ground;
	float3 normal = normalize(cross(ddx(rel), ddy(rel)) + 1e-30);
	normal = dot(normal, dir) > 0 ? -normal : normal;
	float3 cam = cb1[3].xyz;
	// The ground level the heights are taken from: where the ray ends on the ground, else the
	// lowest ground in view (edcas_ground_ref.hlsl). The ray ends on the ground where the depth
	// buffer is as far as the terrain mask: the depth buffer also holds ships, the SRV and
	// plants, and the dust's sheet lay on a parked ship's hull (2026-10-29). Within 2 % and
	// 0.2 m; at 10 % the plants' feet got a hard-edged band of sheet that moved as the
	// commander crouched (2026-10-29). The vehicle's own frame is not ground by the mask either:
	// ground closer than w517 left out as well was a hole in the dust round the outside camera,
	// with a hard edge 10 m off (2026-10-29).
	float4 ref = GroundRef.Load(int3(0, 0, 0));
	float rFloor = d.rPlanet + (ref.y > 0 ? ref.x / ref.y : 0) * (1 - d.rPlanet);
	// The level of the ground nearest the camera in view (edcas_ground_ref.hlsl's second texel): the
	// ground the camera is over, as near as the picture tells. Where none was ever kept, the lowest.
	float4 near = GroundRef.Load(int3(1, 0, 0));
	float rNear = near.y > 0 ? d.rPlanet + near.x / near.y * (1 - d.rPlanet) : rFloor;
	float groundM = ground * d.mPerUnit;
	float maskM = TerrainDist.Load(int3(pixel, 0));
	bool terrain = abs(maskM - groundM) < 0.2 + 0.02 * groundM;
	if (IniParams.Load(int2(516, 0)).w == 2)
		return float4(Scene.Load(int3(pixel, 0)).rgb, 1);
	if (IniParams.Load(int2(516, 0)).w == 1 && ground >= 0) {
		float e = maskM / max(groundM, 1e-3) - 1;
		return float4(saturate(e * 20), saturate(-e * 20), maskM <= 0, 1) * 0.9;
	}
	bool onGround = ground >= 0 && terrain;
	float rGround = onGround ? length(cam + dir * ground) : rFloor;
	// The sine of the angle the ray meets the terrain at (dustHeight), the terrain's plane taken
	// over the billows' depth (terrainFacing).
	float depth = alt ? DepthAlt.Load(int3(pixel, 0)).x : Depth.Load(int3(pixel, 0)).x;
	float metresPerDepth = groundM / max(depth, 1e-6);
	float facing = max(-dot(dir, normal), 0);
	if (onGround)
		facing = terrainFacing(pixel, alt, groundM, metresPerDepth, slope, slopeX, slopeY, pixelAngle / max(pixelsPerTexel.x, pixelsPerTexel.y) * groundM, facing);
	// None from high up, as the fog.
	float hCam = (length(cam) - rGround) * d.mPerUnit;
	float strength = d.strength * (1 - smoothstep(1500, 3000, hCam));
	// Calm nights: the wind near the ground follows the sun (on Mars the night's is 0.19 to 0.37 of the
	// windiest hour's, and dust was lifted between 10:30 and 16:00 only), so a gust that just reaches
	// the threshold by day does not at night. EDCAS writes the night's dust over the day's (w519: 0.001
	// on Mars, about 0.4 on 7 c); it comes back as the sun climbs to 20 degrees. The sun's height
	// cannot tell the morning from the afternoon, where the measured peak is (DUST-PLAN.md phase 4).
	float3 toSun = -cb1[4].xyz;
	float sunLength = length(toSun);
	float sunHeight = sunLength > 0 ? dot(cam, toSun) / (length(cam) * sunLength) : 1;
	strength *= lerp(d.night, 1, smoothstep(-0.05, 0.35, sunHeight));
	if (strength <= 0)
		return 0;

	// Optical depths, times the strength: the sand of the sheets apart from the fine dust of
	// the billows, which scatter differently (below).
	float tau = 0;
	float tEnd = ground >= 0 ? ground : 1e30;

	// The billows: steps closer together near their stretch's start, as far as DUST_FAR_M along it.
	// The stretch starts where the ray comes within the billows' depth (4 * BILLOW_M) of a level it is
	// measured from (dustHeight): the lowest ground in view, or the ground the ray ends on. From the
	// ground that is the camera itself, as it always was. From a ship some hundred metres up it is the
	// last tens of metres of the ray: the 16 steps were spread from the camera to 400 m, one of them
	// in the layer or none, and beyond 400 m there was no dust at all, so the billows showed only to a
	// camera inside them (the owner, 2026-11-12, flying over them at 124 to 300 m: "you can only see
	// it when the camera is just on the right height"; SHADER-REVIEW-HANDOFF.md W10).
	// Along the layer they fade over DUST_NEAR_M to DUST_FAR_M from the stretch's start, as from the
	// camera before; and from the camera over DUST_SEEN_NEAR_M to DUST_SEEN_FAR_M: on hills kilometres
	// off their puffs, each some pixels wide there, stood as blotches all over the land
	// (ed_frame114647, 2026-11-12); that far the dust is the haze's (edcas_clouds.hlsl dustHaze).
	// How far the camera is over the billows on the ground it is over (rNear): 0 inside them, 1 from
	// HOLLOW_M over their top. Until client 0.43.175 this was taken over the lowest ground in view
	// (rFloor), which is the ground the camera stands on only on level land: on a hill with a valley
	// in view the camera, a few metres over the snow, counted as high over the dust, and the dust was
	// drawn as from above on the ground at its feet, twice as strong: a veil over the ground and
	// puffs on it, which came back two seconds after every move of the camera, as the kept level
	// found the valley again (the owner, 2026-11-13, `ed_frame110635.rdc` with it,
	// `ed_frame114904.rdc` without: forced on and off, each replay is the other's picture).
	// Inside the billows a ray's points are measured from the lowest ground in view up to the ray's
	// last fifth (dustHeight). From over them that is not the
	// ground under the ray: with a trench 100 m deep in view from 200 m over a plateau, every point over
	// the plateau counted as 100 m up, and only the lowest metres of each puff were left where the last
	// fifth takes over; a sheen on the sand without a shape, 0.6 of the dust (the owner, 2026-11-12: "I see
	// no dust on the ground"; `ed_frame169294.rdc`). So from over them the stretch, which then starts where
	// the ray comes within their depth of the ground it ends on, is measured from that ground alone, and
	// its steps are even: closer together near the start they lay in the clear air over the layer.
	float above = smoothstep(4 * BILLOW_M, 4 * BILLOW_M + HOLLOW_M, (length(cam) - rNear) * d.mPerUnit);
	// How far from the terrain's plane at the ray's end counts as ground level (dustHeight): nothing
	// from over the billows.
	float hollow = HOLLOW_M * (1 - above);
	float layer = 4 * BILLOW_M / d.mPerUnit;
	float start = 0;
	float rTop = rFloor + layer;
	float along = dot(cam, dir), over = dot(cam, cam) - rTop * rTop;
	if (over > 0) {
		// Above the layer over the lowest ground: where the ray comes down into it (5 m early: the
		// difference of two squares near 1 is good to a fifth of a metre), or never.
		float disc = along * along - over;
		start = disc > 0 && along < 0 ? max(-along - sqrt(disc) - 5 / d.mPerUnit, 0) : 1e30;
	}
	if (onGround)
		start = min(start, max(tEnd - (4 * BILLOW_M + hollow) / max(facing, 0.05) / d.mPerUnit, 0));
	float reach = min(tEnd, start + DUST_FAR_M / d.mPerUnit);
	// Whether a front lies over the near and over the far end of the billows' stretch (frontAt):
	// its cells are kilometres wide, so between those two it is a straight line, one octave each
	// for the pixel and none in the march. The sheet lies where the ray ends.
	float frontNear = frontAt(d, (cam + dir * (reach > start ? start : 0)) * d.mPerUnit), frontFar = frontAt(d, (cam + dir * reach) * d.mPerUnit);
	float sheet = onGround ? sheetDepth(d, cam, dir, ground, pixelAngle, normal, frontFar) * strength : 0;
	// No sand on a slope in the wind's lee (leeShelter).
	[branch] if (sheet > 0)
		sheet *= leeShelter(d, pixel, alt, groundM, metresPerDepth, slope, slopeX, slopeY, pixelAngle / max(pixelsPerTexel.x, pixelsPerTexel.y) * groundM, max(-dot(dir, d.up), 0.01),
			min(pixel, int2(depthWidth, depthHeight) - 1 - pixel));
	float jitter = hash(float3(floor(float2(pixel) * 0.5), 4.5));
	float t = start;
	// No front at either end: no billows on this ray, and no march (the plan's one saving: on a Mars-like
	// body that is most rays, most of the time). Nor where the ray ends before it reaches the layer, nor
	// where the stretch starts beyond DUST_SEEN_FAR_M, where the billows are faded out: from 200 m up that
	// is most of the land in view, and marched through the layer (`above`) it cost 0.33 ms for nothing
	// (`ed_frame169294.rdc`, 2026-11-12).
	[branch] if ((frontNear > 0 || frontFar > 0) && reach > start && start * d.mPerUnit < DUST_SEEN_FAR_M)
	[loop] for (int i = 1; i <= DUST_STEPS; i++) {
		float u = (i - 1 + jitter) / DUST_STEPS;
		float share = lerp(u * u, u, above);
		float next = start + (reach - start) * share;
		float3 p = cam + dir * next;
		// Over the lowest ground in view, and over the ground where the ray ends along its last
		// fifth (dustHeight); from over the billows over that ground all the way (`above`).
		float h = dustHeight(length(p), rFloor, onGround, rGround, (tEnd - next) * d.mPerUnit, facing, hollow, max(smoothstep(0.8, 1, next / tEnd), above), d.mPerUnit);
		float dt = (next - t) * d.mPerUnit;
		tau += billowAt(d, p, h, dt, lerp(frontNear, frontFar, share)) * dt * (1 - smoothstep(DUST_NEAR_M, DUST_FAR_M, (next - start) * d.mPerUnit))
			* (1 - smoothstep(DUST_SEEN_NEAR_M, DUST_SEEN_FAR_M, next * d.mPerUnit));
		t = next;
	}
	// The far drift, past the billows: steps closer together near its start, not jittered: its
	// patches are filtered to the step's length already, and jittered per pixel the drift along
	// the horizon (kilometres of it low over the ground) was grain (2026-10-30).
	float far = 0;
	float farReach = IniParams.Load(int2(519, 0)).x;
	float farEnd = min(tEnd, (farReach > 0 ? farReach : FAR_REACH_M) / d.mPerUnit), farStart = FAR_FROM_M / d.mPerUnit;
	// Not where the dust haze is drawn (x531, edcas_clouds.hlsl dustHaze): that is this dust, through
	// the whole depth of the air, and it takes the far drift's place.
	if (farEnd > farStart && !(IniParams.Load(int2(531, 0)).x > 0)) {
		t = farStart;
		[loop] for (int j = 1; j <= FAR_STEPS; j++) {
			float u = (j - 0.5) / FAR_STEPS;
			float next = farStart + (farEnd - farStart) * u * u;
			float3 p = cam + dir * next;
			float h = dustHeight(length(p), rFloor, onGround, rGround, (tEnd - next) * d.mPerUnit, facing, HOLLOW_M, smoothstep(0.8, 1, next / tEnd), d.mPerUnit);
			float dt = (next - t) * d.mPerUnit;
			t = next;
			// Twenty scale heights up the layer's exp(-h / FAR_H_M) is under 3e-9: no patch to work out (C1).
			if (h > 20 * FAR_H_M)
				continue;
			float3 pM = p * d.mPerUnit;
			float patch = smoothstep(0.45, 0.75, fbmMoving(pM, driftM(d, 0.8 * d.wind), 1 / FAR_PATCH_M, normalize(pM) * (d.time * 0.005), 3, dt / FAR_PATCH_M));
			far += FAR_DENSITY * patch * exp(-h / FAR_H_M) * dt * smoothstep(FAR_FROM_M, DUST_FAR_M, next * d.mPerUnit);
		}
	}
	float fine = tau * strength * lerp(1, DUST_ABOVE, above);
	float4 tuning = IniParams.Load(int2(518, 0));
	float thick = tuning.z > 0 ? tuning.z : DUST_THICK;
	sheet *= thick;
	fine *= thick;
	// Not thickened: `thick` was tuned for the dust close by; w518 instead.
	fine += far * strength * (tuning.w > 0 ? tuning.w : 1);
	float alpha = (1 - exp(-sheet - fine)) * 0.85;
	if (alpha <= 0.001)
		return 0;

	// Over the ground the dust is lit as the ground under it is, as bright as the picture shows
	// that ground, in the dust's own colour: dust in a ship's shadow was lit by the sun, and its
	// streaks drifted over the dark ground as a moving pattern (2026-10-29). In shade it is dark,
	// at night lit by the lamps, and DUST_BRIGHTER than its sand, more towards the sun (below).
	// Averaged over 12 pixels in a spiral DUST_LIGHT_PX wide (a Vogel disc), so the ground's own fine shading
	// (the ripples) averages out and only its shadows and light remain: from the four pixels under
	// the half-size one the dust, several times as bright, carried the ripples and shimmered with
	// the gusts (2026-10-29). Not on an even grid: the ripples repeat every 6 to 10 pixels, and a
	// grid that far apart would land on the same part of each one.
	// In front of anything else that is solid (a rock, a plant, a hull, a building) lit as the
	// ground round that thing is, from discs of the picture 12, 48 and 192 pixels wide, the
	// narrowest that sees ground first (discTrust): this pass knows no shadows, and lit by the sun
	// the dust glowed as bright patches on every rock lying in a crater's shadow (2026-11-12,
	// ed_frame104419: 18 000 pixels, up to 234 levels over the picture without dust).
	// Against the sky lit by the sun and the sky (and where no disc sees ground at most so: below), and brighter towards
	// the sun by each part's Henyey-Greenstein lobe: fine dust scatters forward (g = 0.6, as Mars's dust), the sheets' sand grains, a
	// tenth of a millimetre, much more evenly (g = 0.2). All at 0.6, a thick sheet towards the
	// sun was 2.8 times as bright as sunlit and glowed white (2026-10-29). A thick layer seen
	// from above sends back phase / 4 of the light a white ground under a high sun does (single
	// scattering, Chandrasekhar), so no brighter than its own sand unless the sun is low or
	// behind it; lit as the fog (0.3 + phase / 4, a quarter of the sky's) it was still white.
	float3 sun = -cb1[4].xyz;
	float3 sunColour = starLightTint() * cb1[8].xyz / max(max(cb1[8].x, cb1[8].y), max(cb1[8].z, 1e-30));
	float r = length(cam);
	float sky;
	float light = sunlight(dot(cam / r, sun), d.rPlanet / r, sky);
	float cosine = dot(dir, sun);
	float phase = (sheet * henyeyGreenstein(0.2, cosine) + fine * henyeyGreenstein(0.6, cosine)) / (sheet + fine);
	float3 radiance = d.colour * sunColour * (light * 0.25 * phase + 0.1 * sky);
	if (ground >= 0) {
		float3 weights = float3(0.2126, 0.7152, 0.0722);
		float4 p18 = IniParams.Load(int2(518, 0));
		float grey = dot(d.colour, weights);
		float3 colour = lerp(d.colour, grey, p18.y > 0 ? p18.y : DUST_PALE);
		// In the star's colour while its light is on the ground (`sky`: 1 by day, 0 at night, when
		// the lamps light it), at the brightness it had: the picture gives the dust its brightness
		// only, and under a red star grey dust lay blue-grey over ice the star had turned orange,
		// while against the sky it was the star's colour (2026-11-11). A white star changes nothing.
		float3 starLit = colour * sunColour;
		colour = lerp(colour, starLit * (grey / max(dot(starLit, weights), 1e-6)), sky);
		if (onGround) {
			// Only pixels that show the ground count, by the test this pixel passed above: one that
			// showed a ship's lamp, hundreds of times as bright as the ground, lit its texel up, and
			// every lamp stood in a cluster of pale squares, the texels within 12 pixels whose spiral
			// met it; they jumped as the camera moved (2026-11-10). A hull darkened the dust beside it
			// the same way.
			float3 found[12];
			float taps = 0;
			[unroll] for (int k = 0; k < 12; k++) {
				float angle = k * 2.39996, radius = DUST_LIGHT_PX * sqrt((k + 0.5) / 12);
				int2 offset = int2(round(radius * float2(cos(angle), sin(angle))));
				int3 tap = int3(pixel + offset, 0);
				float tapDepth = alt ? DepthAlt.Load(tap).x : Depth.Load(tap).x;
				float tapM = tapDepth * metresPerDepth * (1 + dot(alongStep, offset / pixelsPerTexel) / alongPerDepth);
				float tapMaskM = TerrainDist.Load(tap);
				bool on = tapMaskM > 0 && abs(tapMaskM - tapM) < 0.2 + 0.02 * tapM;
				found[k] = on ? Scene.Load(tap).rgb : 0;
				taps += on;
			}
			// A glint among them held down (calmSum).
			float3 lit = calmSum(found, taps);
			// None of them on the ground (a patch of it a few pixels wide, between a ship's parts): this pixel.
			lit = taps > 0 ? lit * (12 / taps) : 12 * Scene.Load(int3(pixel, 0)).rgb;
			float brightness = dot(lit / 12, weights) / max(grey, 1e-3);
			radiance = colour * brightness * (p18.x > 0 ? p18.x : DUST_BRIGHTER) * (0.5 + 0.5 * min(phase, 3));
		} else {
			// `open`: the share no disc has taken yet, left to the sun's light worked out above.
			float3 lit = 0;
			float open = 1;
			[loop] for (int disc = 0; disc < 3 && open > 0.01; disc++) {
				float4 found = groundLightDisc(pixel, 12 * exp2(2.0 * disc), alt, metresPerDepth, slope, slopeX, slopeY);
				float share = open * discTrust(found.w);
				lit += found.rgb / max(found.w, 1) * share;
				open -= share;
			}
			float brighter = (p18.x > 0 ? p18.x : DUST_BRIGHTER) * (0.5 + 0.5 * min(phase, 3));
			float brightness = dot(lit, weights) / max(grey, 1e-3);
			// And no brighter than lit as the thing behind the dust is, by the picture at this pixel: this
			// pass knows no shadows, and that thing's own light does. A ship stood before the low sun, the
			// camera on its shadowed side (the owner, 2026-11-13: "the dust or sparkles go through the ship",
			// `ed_frame92302.rdc`): in its middle no disc reaches ground, and left to the open sun, with its
			// lobe towards the sun, a dust of 0.05 took the hull from 23 of 255 to 109 (without the sun's
			// light 23, without the discs' 109); on its flank the discs see the ground behind the ship, the
			// sun's glare path with its glints, and a dust of 0.12 took it from 38 to 127 (without the discs'
			// light 79). All that dust lies in the ship's own shadow. The discs stay: they keep a veil even
			// across a rock's outline, and a lamp or an engine's glow, hundreds of times the hull, lit its
			// dust into pale squares by this pixel alone (2026-11-10). A dark thing in the sun gets less
			// than its due (its paint counts as its light).
			float3 seen = radiance * open + colour * brightness * brighter;
			float3 own = colour * (dot(Scene.Load(int3(pixel, 0)).rgb, weights) / max(grey, 1e-3)) * brighter;
			radiance = dot(own, weights) < dot(seen, weights) ? own : seen;
		}
	}
	return float4(radiance * alpha, alpha);
}
