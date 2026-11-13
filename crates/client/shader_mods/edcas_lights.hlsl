// Elite's own lights in the cloud and the fog: the ship's and the SRV's lights, the suit's torch,
// hull and settlement lamps light the medium they shine through, so in the dark the fog shows only
// where light is. Included by edcas_clouds_final.hlsl after edcas_clouds.hlsl and edcas_fog.hlsl,
// whose helpers it uses.
//
// The lights are Elite's own light records, copied by edcas.ini from its light-volume draws earlier
// in the same frame (no draw of ours there: a capture shader drawing in Elite's light pass washed
// the main menu's hangar out, 2026-10-28), with each draw's light count and its view constants
// (README-ELITE.md, "Lights"). Each light is lit as Elite lights a surface (its light-volume
// shader a5d9ece9c5ceccf7): colour x the cone saturate((cos - outer) / (inner - outer))^exponent
// x the window (1 - (d^2 / range^2)^2)^2 / max(d^2, source radius^2), nothing within the near
// cut-off; times the fog's extinction and an even phase function 1 / (4 pi), on the scene's
// scale cb1[90].y (1/256, as Elite's lighting; Elite exposes the picture after). Elite's shadow
// maps are left out: a beam lights the fog behind a rock.
// The cloud slab is left out of the lamps' light too: where it lay on the ground it scattered a
// hundredth of the fog's, and its density cost most of the time (116 -> 86 FPS, 2026-10-28).
//
// A view-aimed cone with a light flag from EDCAS came first; it followed head look (2026-10-28).

#include "edcas_lamps.hlsl"

// The lights a ray takes along (the ship has four spots; hull lamps reach a few metres), and the
// samples per lamp.
#define LIGHTS_MAX 8
#define LIGHT_STEPS 12
// The width a lamp is blurred by, in texels of this buffer at the sample's distance (see the
// sampling below). 1 left a pair of 2 m hull lamps 500 m off 1.34 times brighter in one camera
// position than in another a fraction of a texel aside, 1.25 1.2 times, 1.5 1.05 times, at 13 %
// less glow from 40 m (a model with the lamp records of FrameAnalysis-2026-10-28, 2026-11-10).
#define LIGHT_FOOTPRINT 1.25

float3 lightsInMedium(float3 v0, float3 v1)
{
	// The angle one texel of this buffer covers, before any branch (a derivative).
	float3 rayAcross = ddx(normalize(v1)), rayDown = ddy(normalize(v1));
	float footprint = LIGHT_FOOTPRINT * max(length(rayAcross), length(rayDown));
	uint spots, spots2, small;
	lampCounts(spots, spots2, small);
	// w509 scales the light, a share of the physical glow (EDCAS's Lamp Glow in Fog; 0 = none):
	// Elite's exposure at night is far from physical, and the physical glow of its 4.8-million-strong
	// ship's lights in mist of 2.4 km sight was white (10 % looked right, 2026-10-28). z509 = 5 draws
	// markers instead, a debug aid: a red dot at each lamp as decoded, a green line along each spot's
	// beam (yellow for the small lamps), to check the decoding against Elite's own lamps.
	float4 tune = IniParams.Load(int2(509, 0));
	uint part = (uint)(tune.z + 0.5);
	if (spots + spots2 + small == 0)
		return 0;
	float3 dir;
	int2 pixel;
	float ground = viewRay(v0, v1, dir, pixel);
	// cb1[19..21] turn a camera-relative world vector (metres) into the planet's frame.
	float unitsPerMetre = length(cb1[19].xyz);
	if (!(unitsPerMetre > 0))
		return 0;
	float3 ray = normalize(v1);
	float far = ground >= 0 ? ground / unitsPerMetre : 1e30;
	float3 camera = lampCamera();

	// z509 = 5: the lamps as this shader sees them, no medium: a red dot at each lamp, a green line
	// 60 m along each spot's beam (yellow for the small lamps), to compare with Elite's own.
	if (part == 5) {
		float3 marks = 0;
		[loop] for (uint m = 0; m < spots + spots2 + small; m++) {
			Light l = lamp(m, spots, spots2, camera);
			float along = dot(l.at.xyz, ray);
			float miss = length(l.at.xyz - ray * along);
			if (along > 0 && miss < 0.05 + 0.004 * along)
				marks = float3(1, 0, 0);
			// Closest approach of the view ray and the beam's axis.
			float3 w0 = -l.at.xyz;
			float b = dot(ray, l.aim.xyz), d = dot(ray, w0), e = dot(l.aim.xyz, w0);
			float denom = max(1 - b * b, 1e-6);
			float tr = (b * e - d) / denom, sa = (e - b * d) / denom;
			float gap = length(w0 + ray * tr - l.aim.xyz * sa);
			if (tr > 0 && sa > 0 && sa < 60 && gap < 0.03 + 0.003 * tr && all(marks == 0))
				marks = m < spots + spots2 ? float3(0, 1, 0) : float3(1, 1, 0);
		}
		return marks * 0.5;
	}

	// No glow asked for (Lamp Glow in Fog at 0) or no fog here: nothing to light, and the lamps
	// are not even read. Before 2026-11-10 every texel went through all of them first (46 at a
	// parked ship) and found the fog missing after.
	if (!(tune.w > 0))
		return 0;
	Fog f;
	if (!loadFog(f))
		return 0;

	// The lights whose reach the ray passes through, tested on each record's place and range before
	// the rest of it is decoded: 46 lamps beside a parked ship, at most LIGHTS_MAX of them taken
	// (PERFORMANCE.md C6, 2026-11-10: every one was decoded in full for every texel).
	Light lights[LIGHTS_MAX];
	int count = 0;
	float reach = 0;
	uint total = spots + spots2 + small;
	[loop] for (uint i = 0; i < total && count < LIGHTS_MAX; i++) {
		float4 at = lampAt(i, spots, spots2, camera);
		float r = at.w;
		if (!(r > 0) || r > 1e5)
			continue;
		float along = dot(at.xyz, ray);
		float miss2 = dot(at.xyz, at.xyz) - along * along;
		if (miss2 >= r * r)
			continue;
		reach = max(reach, along + sqrt(r * r - miss2));
		lights[count] = lamp(i, spots, spots2, camera);
		count++;
	}
	// No fog anywhere the samples go (the sun high over all of it, edcas_fog.hlsl `fogWithin`): no glow.
	if (count == 0 || !fogWithin(f, reach))
		return 0;

	// Per lamp, the stretch of the ray inside its range and in front of the ground, sampled evenly
	// by the angle under which the lamp sees it (equiangular sampling): the samples crowd where
	// the ray passes the lamp, and the light's 1 / d^2 cancels against their spacing, so a few
	// fixed samples give a smooth result. Sixteen random steps along the ray left grain near the
	// lamps and a stepped edge where the ground cut the rays short (2026-10-28).
	//
	// Two things a hull lamp needs on top (2026-11-10; its cone is a pencil, outer cosine 0.99, 9 to
	// 28 cm thick over a range of 2 m):
	// - The samples go only where the lamp shines: the stretch is cut to the inside of the cone, and
	//   split at the near cut-off's sphere (half the samples on each side). Twelve samples over the
	//   whole range missed the pencil by up to 69 % of its glow from 15 m; over the lit stretch they
	//   are within 0.5 %.
	// - The lamp is blurred by the texel: one ray per half-size texel hits such a pencil or misses it
	//   from where it is thinner than the texel (about 100 m), each hit a soft square of the upscale
	//   (in a model; not seen in the game). So the cone's outer edge is widened by
	//   the angle the texel's width has from the lamp, added in 1 - cos (in squares of the angles,
	//   which leaves a cone the texels resolve alone), the cone is dimmed by the share its solid
	//   angle grew, and the source's radius is at least the texel's width.
	float3 cam = cb1[3].xyz;
	float sigmaCamera = f.strength * 0.01 * fogDensity(f, cam);  // per metre
	float3 sum = 0;
	[loop] for (int k = 0; k < count; k++) {
		Light l = lights[k];
		float r = l.at.w;
		float along = dot(l.at.xyz, ray);
		// From the lamp to the ray's nearest point; s runs along the ray from there.
		float3 nearest = ray * along - l.at.xyz;
		float h = max(length(nearest), 1e-2);
		float chord = sqrt(max(r * r - h * h, 0));  // half the chord through the range ("half" is a type)
		float s0 = max(-chord, -along), s1 = min(chord, far - along);
		if (s1 <= s0)
			continue;
		// The cone at its widest on this stretch: the widest texel at the least distance from the lamp.
		float outer = l.aim.w;
		float least = max(h, sqrt(l.cut.x));
		float widest = footprint * (along + s1);
		float c = outer + least * rsqrt(least * least + widest * widest) - 1;
		if (c > 0) {
			// cos >= c: (m s + n)^2 >= c^2 (s^2 + h^2), on the side the lamp aims to.
			float m = dot(ray, l.aim.xyz), n = dot(nearest, l.aim.xyz);
			float qa = m * m - c * c;
			qa = abs(qa) < 1e-6 ? -1e-6 : qa;
			float qb = m * n, qc = n * n - c * c * h * h;
			float disc = qb * qb - qa * qc;
			float root = sqrt(max(disc, 0));
			float lo = (-qb - root) / qa, hi = (-qb + root) / qa;
			if (qa < 0) {
				// The ray crosses the cone: in at one root, out at the other (hi < lo here).
				if (disc <= 0 || m * (lo + hi) * 0.5 + n <= 0)
					continue;
				s0 = max(s0, hi);
				s1 = min(s1, lo);
			} else if (m > 0) {
				// The ray runs inside the cone's angle: it enters and stays.
				s0 = max(s0, max(lo, hi));
			} else {
				s1 = min(s1, min(lo, hi));
			}
			if (s1 <= s0)
				continue;
		}
		// Nothing within the near cut-off: the stretch in front of its sphere and the one behind.
		float cutChord = sqrt(max(l.cut.x - h * h, 0));
		float e0 = s1, e1 = s1;  // the second stretch, empty unless the sphere splits the first
		if (cutChord > 0) {
			e0 = max(s0, cutChord);
			s1 = min(s1, -cutChord);
		}
		bool first = s1 > s0, second = e1 > e0;
		if (!first && !second)
			continue;
		int steps = first ? (second ? LIGHT_STEPS / 2 : LIGHT_STEPS) : 0;
		float a0 = atan(s0 / h), b0 = atan(e0 / h);
		float aStep = first ? (atan(s1 / h) - a0) / steps : 0;
		float bStep = second ? (atan(e1 / h) - b0) / (LIGHT_STEPS - steps) : 0;
		// The cone's solid angle over 2 pi, as Elite gave it.
		float power = max(l.cut.z - outer, 1e-4) / (l.colour.w + 1) + 1 - l.cut.z;
		float3 light = 0;
		[loop] for (int i = 0; i < LIGHT_STEPS; i++) {
			bool front = i < steps;
			// Each sample stands for one step of angle; dt / d^2 = da / h.
			float angle = front ? a0 + aStep * (i + 0.5) : b0 + bStep * (i - steps + 0.5);
			float weight = front ? aStep : bStep;
			float s = h * tan(angle);
			float t = along + s;
			float d2 = h * h + s * s;
			if (d2 < l.cut.x)
				continue;
			float width = footprint * t;
			float edge = max(outer + sqrt(d2) * rsqrt(d2 + width * width) - 1, -1);
			float span = max(l.cut.z - edge, 1e-4);
			float cosine = dot(nearest + ray * s, l.aim.xyz) * rsqrt(d2);
			float cone = pow(saturate((cosine - edge) / span), l.colour.w);
			if (cone <= 0)
				continue;
			cone *= power / (span / (l.colour.w + 1) + 1 - l.cut.z);
			float source2 = max(d2, l.cut.y);
			float near2 = max(source2, width * width);
			float window = max(1 - pow(source2 / (r * r), 2), 0);
			// The fog where the light scatters, and what of it reaches the camera.
			float sigma = f.strength * 0.01 * fogDensity(f, cam + dir * (t * unitsPerMetre));
			light += weight * cone * window * window * (d2 / near2) * sigma * exp(-sigmaCamera * t);
		}
		sum += l.colour.rgb * light / h;
	}
	float3 scattered = sum * (0.25 / 3.14159265) * cb1[90].y * tune.w;
	return all(isfinite(scattered)) ? max(scattered, 0) : 0;
}
