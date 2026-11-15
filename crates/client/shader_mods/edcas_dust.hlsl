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

Texture2D<float> TerrainDist : register(t118);  // .x; its .y is the sunset on the ground, for the sun pass (ground_tint.py mask)
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
struct Dust {
	float strength, wind, time, mPerUnit, rPlanet, sheetM, activity, night;
	float3 colour, up, along;
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
	d.night = p19.w > 0 ? p19.w : 1;
	// The wind along the planet's x axis (y near the x axis' poles), as the clouds' streaks
	// (edcas_clouds.hlsl). The pattern streams along that axis itself: points are planet-centred,
	// about a planet radius long, and along the axis laid flat at the camera (`east`) they moved
	// with the camera, which turns it by 1/R per metre; walking added to the wind or took from
	// it (2026-10-29).
	d.up = normalize(cb1[3].xyz);
	d.along = abs(d.up.x) < 0.95 ? float3(1, 0, 0) : float3(0, 1, 0);
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

// The saltation sheet over the ground where a ray ends: its optical depth, from the camera's
// height down to the ground along the ray, streaked along the wind and stronger in gusts.
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
	// Streamers: noise stretched 2.5 times along the wind, moving at its speed.
	float streak = fbmMoving(pM, driftM(d, d.wind), (1 - 0.6 * d.along) / 1.5, 0, 3, footM / 1.5);
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
	return 0.12 * (SHEET_M / d.sheetM) * near * path * (0.15 * d.activity + gust) * (0.3 + 1.4 * streak);
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
float terrainFacing(int2 pixel, bool alt, float groundM, float metresPerDepth, float2 slope, float2 slopeX, float2 slopeY, float metresPerPixel, float fallback)
{
	float along = sqrt(1 + dot(slope, slope));
	int reach = (int)clamp(2 * BILLOW_M / max(metresPerPixel, 1e-6), 2, 48);
	float3 centre = float3(slope, 1) * (groundM / along);
	float w0 = along / max(groundM, 1e-6);
	float3 span[2];
	[unroll] for (int axis = 0; axis < 2; axis++) {
		float3 side[2];
		float w[2];
		bool on[2];
		[unroll] for (int k = 0; k < 2; k++) {
			int2 offset = (axis == 0 ? int2(reach, 0) : int2(0, reach)) * (2 * k - 1);
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
		if (on[0] && on[1] && abs((w0 - w[0]) - (w[1] - w0)) <= 0.25 * (before + after) + 1e-4 * w0)
			span[axis] = side[1] - side[0];
		else if (on[1] && (!on[0] || after < before))
			span[axis] = side[1] - centre;
		else if (on[0])
			span[axis] = centre - side[0];
		else
			return fallback;
	}
	float3 plane = cross(span[0], span[1]);
	float size = length(plane);
	return size > 0 ? abs(dot(plane, float3(slope, 1))) / (size * along) : fallback;
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
	float2 pixelsPerTexel = max(abs(float2(ddx((float)pixel.x), ddy((float)pixel.y))), 1);
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
	bool alt = asint(cb1[25].w) == 1;
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
	// Averaged over 12 pixels in a spiral 12 wide (a Vogel disc), so the ground's own fine shading
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
				float angle = k * 2.39996, radius = 12 * sqrt((k + 0.5) / 12);
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
