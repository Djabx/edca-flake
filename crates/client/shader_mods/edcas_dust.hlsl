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
// drift (0 = 1), to tune in game. Row 19: x519 the far drift's reach in metres (0 = FAR_REACH_M).
//
// t118: the terrain mask, the ground's distance from the camera in metres where Elite's ground
// shaders drew it (ground_tint.py mask, edcas.ini CommandListEdcasTerrainMask), 0 elsewhere.

Texture2D<float> TerrainDist : register(t118);
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
struct Dust {
	float strength, wind, time, mPerUnit, rPlanet;
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

// The saltation sheet over the ground where a ray ends: its optical depth, from the camera's
// height down to the ground along the ray, streaked along the wind and stronger in gusts.
// `pixelAngle`: the angle one pixel spans (radians), `normal` the terrain's.
float sheetDepth(Dust d, float3 cam, float3 dir, float ground, float pixelAngle, float3 normal)
{
	float3 hit = cam + dir * ground;
	float slope = groundSlope(dir, hit, normal);
	float path = sheetPath(d, dir, hit, ground, SHEET_M, slope);
	// The ground one pixel covers, in metres: stretched along the ray where it meets the
	// ground at a flat angle.
	float footM = ground * d.mPerUnit * pixelAngle / slope;
	float3 pM = hit * d.mPerUnit;
	// Streamers: noise stretched 2.5 times along the wind, moving at its speed.
	float streak = fbmMoving(pM, driftM(d, d.wind), (1 - 0.6 * d.along) / 1.5, 0, 3, footM / 1.5);
	float gust = gustAt(d, pM, driftM(d, 0.8 * d.wind), footM);
	// Gone towards the horizon by the angle the ray meets the planet at, as well as by the
	// distance: on a plain seen from eye height the ground from DUST_NEAR_M to DUST_FAR_M lies
	// in a few pixels' rows just under the horizon, and the fade by distance alone was a hard
	// line there, at eye level, that moved as the commander crouched (2026-10-29). Over 0.2 to
	// 1.1 degrees it spans some tens of rows; over 0.3 to 3 it took the sheet from beyond 35 m on
	// foot and the dust was hardly seen (2026-10-29).
	float near = (1 - smoothstep(DUST_NEAR_M, DUST_FAR_M, ground * d.mPerUnit)) * smoothstep(0.003, 0.02, -dot(dir, normalize(hit)));
	return 0.12 * near * path * (0.15 + gust) * (0.3 + 1.4 * streak);
}

// The billows' density at a point (1 / m), lifted in the gusts. `stepM`: the march's step there
// (metres): puffs and gusts finer than the steps can resolve are averaged out, or each pixel's
// jittered steps would land on different ones and the air would sparkle (2026-10-29).
float billowAt(Dust d, float3 p, float h, float stepM)
{
	if (h > 4 * BILLOW_M)
		return 0;
	float3 pM = p * d.mPerUnit, shiftM = driftM(d, 0.8 * d.wind);
	float gust = gustAt(d, pM, shiftM, stepM);
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

// The dust along a pixel's view ray, premultiplied: in front of the fog and the clouds.
float4 groundDust(float3 v0, float3 v1)
{
	Dust d;
	if (!loadDust(d))
		return 0;
	float3 dir;
	int2 pixel;
	float ground = viewRay(v0, v1, dir, pixel);
	// Taken here, before any branch that differs between pixels: the angle a pixel spans and
	// the terrain's normal, facing the camera, from the hit relative to the camera (precise,
	// where the planet-centred hit is only good to a tenth of a metre).
	float pixelAngle = max(length(ddx(dir)), length(ddy(dir)));
	// For the picture's pixels round this one (the dust's light, below): the length along a ray
	// per metre of view depth, how it changes from texel to texel, and the picture's pixels a texel.
	float alongPerDepth = length(v1) / dot(v1, cb1[279].xyz);
	float2 alongStep = float2(ddx(alongPerDepth), ddy(alongPerDepth));
	float2 pixelsPerTexel = max(abs(float2(ddx((float)pixel.x), ddy((float)pixel.y))), 1);
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
	// None from high up, as the fog.
	float hCam = (length(cam) - rGround) * d.mPerUnit;
	float strength = d.strength * (1 - smoothstep(1500, 3000, hCam));
	if (strength <= 0)
		return 0;

	// Optical depths, times the strength: the sand of the sheets apart from the fine dust of
	// the billows, which scatter differently (below).
	float sheet = onGround ? sheetDepth(d, cam, dir, ground, pixelAngle, normal) * strength : 0;
	float tau = 0;
	float tEnd = ground >= 0 ? ground : 1e30;

	// The billows: steps closer together near the camera, as far as DUST_FAR_M.
	float reach = min(tEnd, DUST_FAR_M / d.mPerUnit);
	float jitter = hash(float3(floor(float2(pixel) * 0.5), 4.5));
	float t = 0;
	[loop] for (int i = 1; i <= DUST_STEPS; i++) {
		float u = (i - 1 + jitter) / DUST_STEPS;
		float next = reach * u * u;
		float3 p = cam + dir * next;
		// Over the lowest ground in view, and over the ground where the ray ends only along the
		// ray's last fifth: taken over a far hill's ground all the way, the air in front of it
		// was at ground level for the whole 400 m and the hill turned white (2026-10-29). Under
		// the ground (rising to eye level and over) taken as at ground level.
		float rRef = lerp(min(rFloor, rGround), rGround, smoothstep(0.8, 1, next / tEnd));
		float h = max((length(p) - rRef) * d.mPerUnit, 0);
		float dt = (next - t) * d.mPerUnit;
		tau += billowAt(d, p, h, dt) * dt * (1 - smoothstep(DUST_NEAR_M, DUST_FAR_M, next * d.mPerUnit));
		t = next;
	}
	// The far drift, past the billows: steps closer together near its start, not jittered: its
	// patches are filtered to the step's length already, and jittered per pixel the drift along
	// the horizon (kilometres of it low over the ground) was grain (2026-10-30).
	float far = 0;
	float farReach = IniParams.Load(int2(519, 0)).x;
	float farEnd = min(tEnd, (farReach > 0 ? farReach : FAR_REACH_M) / d.mPerUnit), farStart = FAR_FROM_M / d.mPerUnit;
	if (farEnd > farStart) {
		t = farStart;
		[loop] for (int j = 1; j <= FAR_STEPS; j++) {
			float u = (j - 0.5) / FAR_STEPS;
			float next = farStart + (farEnd - farStart) * u * u;
			float3 p = cam + dir * next;
			float rRef = lerp(min(rFloor, rGround), rGround, smoothstep(0.8, 1, next / tEnd));
			float h = max((length(p) - rRef) * d.mPerUnit, 0);
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
	float fine = tau * strength;
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
	// Elsewhere (sky, plants, ships behind it) lit by the sun and the sky, and brighter towards
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
	if (onGround) {
		// Only pixels that show the ground count, by the test this pixel passed above: one that
		// showed a ship's lamp, hundreds of times as bright as the ground, lit its texel up, and
		// every lamp stood in a cluster of pale squares, the texels within 12 pixels whose spiral
		// met it; they jumped as the camera moved (2026-11-10). A hull darkened the dust beside it
		// the same way.
		bool alt = asint(cb1[25].w) == 1;
		float depth = alt ? DepthAlt.Load(int3(pixel, 0)).x : Depth.Load(int3(pixel, 0)).x;
		float metresPerDepth = groundM / max(depth, 1e-6);
		float3 lit = 0;
		float taps = 0;
		[unroll] for (int k = 0; k < 12; k++) {
			float angle = k * 2.39996, radius = 12 * sqrt((k + 0.5) / 12);
			int2 offset = int2(round(radius * float2(cos(angle), sin(angle))));
			int3 tap = int3(pixel + offset, 0);
			float tapDepth = alt ? DepthAlt.Load(tap).x : Depth.Load(tap).x;
			float tapM = tapDepth * metresPerDepth * (1 + dot(alongStep, offset / pixelsPerTexel) / alongPerDepth);
			float tapMaskM = TerrainDist.Load(tap);
			if (tapMaskM > 0 && abs(tapMaskM - tapM) < 0.2 + 0.02 * tapM) {
				lit += Scene.Load(tap).rgb;
				taps++;
			}
		}
		// None of them on the ground (a patch of it a few pixels wide, between a ship's parts): this pixel.
		lit = taps > 0 ? lit * (12 / taps) : 12 * Scene.Load(int3(pixel, 0)).rgb;
		float3 weights = float3(0.2126, 0.7152, 0.0722);
		float4 p18 = IniParams.Load(int2(518, 0));
		float grey = dot(d.colour, weights);
		float3 colour = lerp(d.colour, grey, p18.y > 0 ? p18.y : DUST_PALE);
		float brightness = dot(lit / 12, weights) / max(grey, 1e-3);
		radiance = colour * brightness * (p18.x > 0 ? p18.x : DUST_BRIGHTER) * (0.5 + 0.5 * min(phase, 3));
	}
	return float4(radiance * alpha, alpha);
}
