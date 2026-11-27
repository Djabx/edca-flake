// The light of the post effects' lens flare, anamorphic streaks, diffraction spikes and lens dirt (README.md "Post
// effects", 2026-11-26), drawn into ResourceEdcasPostLensLight at a quarter of the picture's size in linear light;
// edcas_post_lens.hlsl adds it. EDCAS's own.
//
// Only a star lights the lens (2026-11-26). A lens ghost is light reflected twice between coated glass surfaces, a
// ten-thousandth or less of its source (Hullin et al., "Physically-Based Real-Time Lens Flare Rendering", 2011), so
// only a source vastly brighter than the scene leaves one: a sun (some 1e9 cd/m2) does, a lamp (1e4 to 1e6) or the
// Milky Way does not. Elite's HDR scene does not carry that: in ed_frame27784 the sun's disc is 5, a cockpit
// decoration lamp 120, and the Milky Way's core in the bloom (ed_frame47500) outshines the sun's disc too. Taken
// from the brightness (Elite's bloom over a threshold, until 0.43.281) the lamps and the Milky Way flared before a
// sun did (the owner). So the light is the picture where Elite's star surface shader drew (edcas.ini
// ResourceEdcasStarMask, edcas_star_mask.hlsl; what stands before the star is not in it):
// - STAR_LIGHT: the star's light at a quarter of the picture's size, each texel the mean of its 4 x 4 pixels
//   (ResourceEdcasPostStarLight), which the other passes read;
// - DIRT_LIGHT, DIRT_H, DIRT_V: the same at a 32nd, blurred wide (the glow, which the dirt glows with);
// - FLARE_GRID, FLARE_SUM: the 32nd before its blur summed, for the lens flare's ghosts (LensFlare);
// - the pass itself: the four effects.
// The picture holds a star's disc near white whatever the star, so the light says how much of the lens a star
// covers, not how bright it is: a far sun's few pixels would leave next to nothing and a near one flood the lens.
// The square root of the gathered light stands between the two (Compress); the ghosts take the cube root of the
// total (LensFlare).
#include "edcas_post.hlsl"

#if defined(STAR_LIGHT)
// Where a star's surface was drawn this frame (1), at the picture's size.
Texture2D<float4> StarMask : register(t117);

float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
	float2 px = PictureSize().zw;
	float3 sum = 0;
	[unroll] for (int y = 0; y < 4; y++)
		[unroll] for (int x = 0; x < 4; x++)
		{
			int2 at = int2((uv + (float2(x, y) - 1.5) * px) * PictureSize().xy);
			sum += ToLinear(Picture.Load(int3(at, 0)).rgb) * StarMask.Load(int3(at, 0)).r;
		}
	return float4(sum / 16, 1);
}
#else

// The star's light at a quarter of the picture's size (STAR_LIGHT above).
Texture2D<float4> StarLight : register(t113);

// The star's light at uv; 0 outside the picture.
float3 BrightAt(float2 uv)
{
	if (any(uv < 0) || any(uv > 1))
		return 0;
	return BilinearAt(StarLight, uv).rgb;
}

// Between a far sun and a near one (above): the square root of a gathered light's brightness, its colour kept.
float3 Compress(float3 light)
{
	float l = max(light.r, max(light.g, light.b));
	return l > 1e-6 ? light * (sqrt(l) / l) : 0;
}

// The star's light blurred wide: a 32nd of the picture's size (DIRT_LIGHT below), then a Gaussian of sigma 4
// texels across and down (DIRT_H, DIRT_V; some 130 pixels at 2160 lines). The dirt glows with it. FLARE_GRID reads
// it before its blur.
Texture2D<float4> Glow : register(t116);

// The glow fades out towards the picture's edge inside the ellipse that touches its four sides, as a round lens
// cuts its light off: nothing of the picture's straight edges is left in the dirt's glow.
float3 GlowAt(float2 uv)
{
	float inside = smoothstep(1.0, 0.55, length((uv - 0.5) * 2));
	if (inside <= 0)
		return 0;
	return BilinearAt(Glow, uv).rgb * inside;
}

// The lens flare (round 3, 2026-11-26): layers, as games compose one (Unity's Lens Flare (SRP) elements, Autodesk
// VRED's, the Battlefield 3 and EVE Online breakdowns), each kept to what a lens does (Hullin et al. 2011, Lee and
// Eisemann 2013):
// - at the star: a soft glow (scatter in the glass) and a starburst of thin rays of uneven length (diffraction at
//   the iris's blades), both following the star light's total;
// - ghosts: light reflected twice between lens surfaces, an image of a stop put on the line through the picture's
//   centre (a point at p gives the ghost at -k p, k its own). Mostly small and hollow: a stop's image is brightest
//   at its edge (the rim, where the reflected wavefront folds into a caustic), its inside faint. Round by default
//   (IRIS_BLADES): a lens wide open, or with rounded blades, images a round stop; half the ghosts are the iris's
//   images and turn into polygons only with IRIS_BLADES 3 or more, the others are round rims of other elements.
//   Rims in the coatings' colours, red outside blue (dispersion), insides near the star's own colour;
// - a halo: a thin ring round the picture's centre from a ghost focused near the stop, split into a rainbow by
//   dispersion (red outside), seen as the arc on the star's side.
// The source is the star light's sum (FLARE_SUM below): its total, its light-weighted centre and spread. An
// extended star spreads each ghost by its own picture scaled by k: rims widen and fade (their light is kept, not
// their peak). Two stars in view at once are taken as one light between them.
#define GHOSTS 6
// Unlike each other on purpose (round 5): unevenly spaced, of distinctly different sizes and kinds. Round 8: the four
// ring ghosts had grown alike, so their radii are some 1 : 1.6 : 2.5 apart (0.016 to 0.040 picture heights) and they
// differ in kind: a thin-rimmed one, a plain one, a faint evenly filled disc with a slightly brighter edge, and a
// small soft one; then a large faint one and one very large, barely there.
static const float ghostK[GHOSTS] = { -0.45, -0.18, 0.15, 0.34, 0.70, 0.95 };
static const float ghostR[GHOSTS] = { 0.016, 0.060, 0.026, 0.040, 0.022, 0.140 };	// picture heights
static const float ghostRim[GHOSTS] = { 0.8, 0.25, 0.8, 0.35, 0.55, 0.06 };	// the rim's brightness
static const float ghostRimW[GHOSTS] = { 0.04, 0.07, 0.07, 0.07, 0.10, 0.07 };	// the rim's half width, of the radius
static const float ghostLevel[GHOSTS] = { 0.08, 0.4, 0.2, 0.8, 0.3, 0.4 };	// the inside's level, of the rim's
static const int ghostIris[GHOSTS] = { 1, 0, 1, 0, 1, 0 };	// 1 the iris's image, 0 another element's round rim
static const int ghostCoat[GHOSTS] = { 0, 1, 2, 0, 1, 2 };
// How far the star's own width spreads each ghost: 1 as the optics of an extended source has it (the star's
// picture scaled by k); 0.3 for the two small bright ones, so their rim stays defined but not hard before a large
// sun (round 6 gave them 0, crisp, round 7 0.3: one family with the others). A choice for the look, not the optics,
// which would spread them as the others.
static const float ghostSpread[GHOSTS] = { 0.3, 1, 1, 0.3, 1, 1 };
// the coatings' tints on the star's colour (luminance about 1), slight (round 5: the saturated green and magenta of
// round 3 turned the ghosts of a large orange sun plainly green and pink): amber, teal, violet
static const float3 coatTint[3] = { float3(1.10, 1.00, 0.80), float3(0.90, 1.04, 1.08), float3(1.06, 0.94, 1.12) };

// The star light's sum (FLARE_SUM): texel 0 its colour (the mean over the picture), texel 1 its light-weighted
// centre (xy, picture heights from the centre) and spread (z, sigma along one axis).
Texture2D<float4> Flare : register(t119);
// The star light over a 16 x 8 grid of cells (FLARE_GRID below), two texels a cell: the colour summed (rgb) and
// its luminance (a); the luminance times the place (xy) and times the place's square (z).
Texture2D<float4> FlareGrid : register(t118);

// The iris's blades as its ghosts show them: 0 (or fewer than 3) round, a lens wide open or with rounded blades
// (the default: the look photographers prefer, and a camera on a dark scene shoots wide open); 6 the six-bladed
// iris's hexagon (client 0.43.284 to 0.43.286). One constant, for a later setting.
#ifndef IRIS_BLADES
#define IRIS_BLADES 0
#endif

// A point's distance from a ghost's centre in the iris's measure: 1 on the polygon's corners, a fifth of it round
// (the blades' curve); with no blades the plain distance. All the iris's ghosts are turned alike: a ghost is the
// iris's image, upright or inverted (the same polygon for an even blade count).
float IrisDistance(float2 d)
{
#if IRIS_BLADES >= 3
	const float n = IRIS_BLADES, sector = 6.2831853 / n;
	float a = atan2(d.y, d.x) - 0.2618;	// a flat 15 degrees off the axes
	float w = a - sector * floor(a / sector) - sector / 2;
	float flat = length(d) * cos(w);	// the distance along the nearest flat's normal
	return flat / cos(sector / 2) * 0.8 + length(d) * 0.2;
#else
	return length(d);
#endif
}


// The glow and the starburst at the star: what the visible star light makes of the lens's scatter and of
// diffraction at the iris's blades. Both are the source's light convolved with a kernel, not drawn from its middle:
// every bright point casts them, so a far sun (a point) gets a crisp star of rays and a soft glow, a big disc only
// a soft fringe beyond its limb (its rays smear into each other), and what hides the star (a ship, the canopy's
// frame) casts nothing. The source is the 16 x 8 grid of the star light (FLARE_GRID), each cell a uniform disc at
// its light's centre with its light's spread; only the 5 x 5 cells round the pixel are read (the kernels reach
// some 0.15 picture heights, read to two cells).
// The rays: each straight blade edge diffracts a ray both ways, so an even count of blades n gives n rays and an
// odd one 2n. STAR_BLADES 9, the rounded nine-bladed iris common in modern lenses: 18 faint rays (rounded blades
// diffract little), fixed to the lens, each a little longer or shorter and stronger or weaker (the blades are not
// alike). A ray from a disc through the pixel x along u is the kernel exp(-t / length) integrated over the chord
// the line x - t u cuts through the disc, its width the disc's (at least the ray's own, wr).
// x: the pixel (picture heights from the centre); returns the glow (rgb) and rays (rgb) per unit of the total's
// luminance (lum).
#define STAR_BLADES 9
#define RAY_REACH 0.15
#define GLOW_RADIUS 0.04

float FlareHash(float n)
{
	return frac(sin(n * 12.9898) * 43758.5453);
}

void StarGlowRays(float2 x, float aspect, float wr, float lum, out float3 glow, out float3 rays)
{
	glow = 0;
	rays = 0;
	const int N = (STAR_BLADES % 2) ? 2 * STAR_BLADES : STAR_BLADES;
	const int LINES = N / 2;
	float2 cellSize = float2(aspect / 16, 1.0 / 8);
	int2 here = int2(floor((x + float2(aspect, 1) * 0.5) / cellSize));
	float texel;
	{
		float w, h;
		Glow.GetDimensions(w, h);
		texel = 1 / h;
	}
	[loop] for (int cy = max(here.y - 2, 0); cy <= min(here.y + 2, 7); cy++)
		[loop] for (int cx = max(here.x - 2, 0); cx <= min(here.x + 2, 15); cx++)
		{
			float4 a = FlareGrid.Load(int3(2 * cx, cy, 0));
			if (a.a <= 1e-9)
				continue;
			float4 b = FlareGrid.Load(int3(2 * cx + 1, cy, 0));
			float2 c = b.xy / a.a;
			float variance = max((b.z / a.a - dot(c, c)) / 2 - texel * texel / 12, 0);
			float2 v = x - c;
			// the glow: a Gaussian of GLOW_RADIUS round each point, so round the cell's disc one as wide as both
			float g2 = GLOW_RADIUS * GLOW_RADIUS + variance;
			glow += a.rgb * exp(-dot(v, v) / (2 * g2)) * (GLOW_RADIUS * GLOW_RADIUS / g2);
			// the rays
			float re = max(2 * sqrt(variance), wr);	// the disc's radius (a uniform disc's sigma is half of it)
			float ray = 0;
			[unroll] for (int j = 0; j < LINES; j++)
			{
				float th = 0.2618 + j * 6.2831853 / N;
				float2 u = float2(cos(th), sin(th));
				float t0 = dot(v, u), hh = v.x * u.y - v.y * u.x;
				if (abs(hh) >= re)
					continue;
				float q = sqrt(re * re - hh * hh);
				// the ray going +u (the source behind the pixel, t > 0) and the one going -u (t < 0)
				float lp = RAY_REACH * lerp(0.6, 1.0, FlareHash(j + 7)), ln = RAY_REACH * lerp(0.6, 1.0, FlareHash(j + LINES + 7));
				float ip = lp * (exp(-max(t0 - q, 0) / lp) - exp(-max(t0 + q, 0) / lp));
				float in_ = ln * (exp(-max(-q - t0, 0) / ln) - exp(-max(q - t0, 0) / ln));
				ray += ip * lerp(0.5, 1.0, FlareHash(j + 31)) + in_ * lerp(0.5, 1.0, FlareHash(j + LINES + 31));
			}
			rays += a.rgb * ray * wr / (3.1415927 * re * re);
		}
	glow /= lum;
	rays /= lum;
}

// A ghost's light (its profile summed over the picture, in units of its rim's brightness): the inside, a flat level
// rising to the rim over 2.5 rim widths, and the Gaussian falloff outside the radius.
float GhostLight(float r, float w, float level)
{
	float flat = max(r - 2.5 * w, 0) / r;
	return 3.1415927 * (r * r * (level + (1 - level) * 0.5 * (1 - flat * flat)) + r * w * 1.7725 + w * w);
}

// A dim yellow on black reads olive green (round 9, the game): the ghosts of a yellow or orange star lean a little
// warm, red up and blue down at about the same luminance, the more the yellower the star (its red over its blue);
// a white or blue-white star's ghosts are left as they are, cool stays cool.
#define GHOST_WARM float3(1.10, 0.96, 0.82)

float3 LensFlare(float2 uv, float aspect, float aa, float spikes)
{
	float3 light = Flare.Load(int3(0, 0, 0)).rgb;
	float4 shape = Flare.Load(int3(1, 0, 0));
	float l = max(light.r, max(light.g, light.b));
	if (l <= 1e-7)
		return 0;
	// between a far sun and a near one: the cube root of the total (a near sun's light measures its area, not
	// its brightness: the picture holds every star's disc near white), its colour kept
	float lum = pow(max(dot(light, float3(0.2126, 0.7152, 0.0722)), 0), 1.0 / 3);
	float3 star = light / l;
	float2 x = (uv - 0.5) * float2(aspect, 1);
	float2 s = shape.xy;
	float sigma = shape.z;
	float out_ = length(s);
	// off the centre the ghosts spread out and the halo shows; on it they stand behind the star
	float offc = smoothstep(0.05, 0.7, out_);
	float3 sum = 0;

	// the glow and the starburst, from the visible star light (StarGlowRays)
	float3 glow, rays;
	StarGlowRays(x, aspect, 1.2 * aa, dot(light, float3(0.2126, 0.7152, 0.0722)), glow, rays);
	sum += glow * 0.1 + rays * 0.19 * (1 - 0.65 * spikes);

	// the ghosts. Fainter as the star's visible area grows (round 9): the total's cube root lets a large sun's ghosts
	// carry far more light than their smeared area holds see-through (opaque discs over the stars and the HUD in
	// the game). A steep fall over the area (the total light, the share of the picture a near-white disc covers):
	// a sliver or a medium sun (under 1 % of the picture) keeps nearly all, one covering 8 to 10 % under 1 %. Taken
	// from the area, not the spread: a sliver at the picture's edge is small but spread wide. A choice for the look.
	float area = dot(light, float3(0.2126, 0.7152, 0.0722)) / 0.015;
	float sizeFade = 1 / (1 + area * area * area);
	float3 warm = lerp(1, GHOST_WARM, saturate((star.r - star.b) / 0.4));
	[unroll] for (int i = 0; i < GHOSTS; i++)
	{
		float k = ghostK[i];
		float2 g = x + k * s;
		float r0 = ghostR[i];
		float rho = 2 * abs(k) * sigma * ghostSpread[i];	// the mirrored star's radius (a uniform disc's sigma is half its radius)
		// a polygon spread by a star's picture wider than itself turns round
		float dist = ghostIris[i] ? lerp(IrisDistance(g), length(g), rho / (r0 + rho)) : length(g);
		// near the centre the ghosts stand behind the star; the two kept sharper would stand out on whatever hides
		// its middle (a ship), so they fade out there entirely
		float place = ghostSpread[i] >= 1 ? 0.4 + 0.6 * offc : offc;
		float3 rgb;
		[unroll] for (int ch = 0; ch < 3; ch++)
		{
			float r = r0 * (1.04 - 0.04 * ch);	// red outside, blue inside
			float w = max(ghostRimW[i] * r, 0.5 * aa);	// the rim's half width, sharp (some 3 pixels at least)
			float wide = sqrt(w * w + 0.36 * rho * rho);	// spread by the star's picture
			// outside the rim a Gaussian falloff; inside, a rise from a flat level in the middle (its slope 0 there:
			// round 7 let the rim's Gaussian run on to the centre, whose cone-shaped low point read as a dark dot)
			float t = (dist - r) / wide;
			// a ring smeared by a star's picture about as wide as itself fills in: its middle rises to its rim's light
			float level = lerp(ghostLevel[i], 1, smoothstep(0.25, 0.9, wide / r));
			float inside = lerp(level, 1, smoothstep(max(r - 2.5 * wide, 0), r, dist));
			float shape = dist < r ? inside : exp(-t * t);
			// its light kept as the star's width spreads it: scaled by the crisp shape's light over the spread
			// one's (round 9: round 8 filled a smeared ring in at its rim's brightness, a disc with far more light
			// than the ring, opaque olive discs before a large sun in the game)
			rgb[ch] = shape * GhostLight(r, w, ghostLevel[i]) / GhostLight(r, wide, level);
		}
		sum += rgb * star * coatTint[ghostCoat[i]] * warm * ghostRim[i] * place * sizeFade;
	}

	// the halo: a ghost focused near the stop, a faint soft band of 0.45 picture heights round the centre, nudged
	// away from the star, on the star's side. Round 7: a hint, not a shape (a third of round 6's light), an exact
	// circle of constant width, in the star's own colour (round 6's third of dispersion fringe put the green
	// channel's ring in the band's middle: grey-green next to an orange star in the game); only its brightness
	// changes along the arc, in one or two broad swells, fading to nothing some 65 degrees to either side
	float2 h = x + 0.08 * s;
	float hd = length(h);
	float2 hn = h / max(hd, 1e-5), sn = s / max(out_, 1e-5);
	float phi = atan2(hn.x * sn.y - hn.y * sn.x, dot(hn, sn));	// the angle from the star's side
	float along = smoothstep(1.15, 0.1, abs(phi)) * (0.6 + 0.4 * sin(1.7 * phi + 0.6));
	float hw = 0.03 + 0.02 * sigma;
	float ht = (hd - 0.45) / hw;
	sum += star * exp(-ht * ht) * 0.007 * along * offc;
	return sum * lum;
}

// Anamorphic streaks: the star's light smeared far to both sides (a cinema lens's cylinder spreads it
// horizontally), blue as such streaks mostly are. 48 reads a side, 5 pixels apart at 1440 lines.
float3 Streaks(float2 uv, float2 step)
{
	float3 sum = 0;
	[loop] for (int i = 1; i <= 48; i++)
	{
		float x = i * 2.5 * step.x;
		sum += exp(-i / 7.0) * (BrightAt(uv + float2(x, 0)) + BrightAt(uv - float2(x, 0)));
	}
	float l = sqrt(dot(sum, float3(0.2126, 0.7152, 0.0722)) / 13.0);
	return l * float3(0.45, 0.7, 1.0);
}

// Diffraction spikes: the star a camera's aperture puts on a bright light, from the straight edges of its blades.
// Six spikes (three lines), as from a six-bladed aperture or JWST's hexagonal mirror, turned 15 degrees off the
// horizontal so they do not lie on the anamorphic streaks. 24 reads a side, 6 pixels apart at 1440 lines.
// A spike's length scales with the wavelength (the diffraction angle is lambda / d): read at three scales, red
// reaching furthest and blue least, its tip runs into a faint rainbow. That split only shows on the thin ray of a
// point-like source; where the rays of a wide disc overlap the colours cancel into the star's own. So (2026-11-26,
// client 0.43.296; game screenshots: a large sun's spikes were a thick glowing band round its limb with a green and
// purple outer rim, and veiled its surface) the ray is the star-coloured one read at one length, the split blended
// in only as far as the source is point-like (spread: the star light's sigma, picture heights).
float3 Spikes(float2 uv, float2 step, float aspect, float spread)
{
	float3 sum = 0;
	float split = 1 / (1 + spread * spread / (0.008 * 0.008));
	const float2 dirs[3] = { float2(0.9659, 0.2588), float2(-0.2588, 0.9659), float2(-0.7071, -0.7071) };
	[unroll] for (int d = 0; d < 3; d++)
	{
		float3 line_ = 0, plain = 0;
		float2 o1 = dirs[d] * 3.0 * float2(step.x, step.x * aspect);
		[loop] for (int i = 1; i <= 24; i++)
		{
			float w = exp(-i / 8.0);
			float2 o = o1 * i;
			float3 mid = BrightAt(uv + o) + BrightAt(uv - o);
			plain += w * mid;
			// 650, 550 and 450 nm against green's 550: red's spike 1.18 times as long, blue's 0.82
			line_.r += w * (BrightAt(uv + o / 1.18).r + BrightAt(uv - o / 1.18).r);
			line_.g += w * mid.g;
			line_.b += w * (BrightAt(uv + o / 0.82).b + BrightAt(uv - o / 0.82).b);
		}
		sum += Compress(lerp(plain, line_, split) / 15.0);
	}
	return sum;
}

// The spikes of a star that shows as a small disc (client 0.43.298): summed from every point of the disc the rays
// above make bars as wide as the disc, four fat lobes like a propeller on a sun of some tens of pixels. Drawn
// instead from the star light's centre s (picture heights): each ray starts at the limb (radius r, twice the
// light's sigma), thin there and narrowing to a point as it fades over some 0.05 picture heights, in the star's
// colour (a disc's rays overlap too much for a spectral split). x: the pixel; aa: some 6 pixels.
float3 DiscSpikes(float2 x, float2 s, float r, float aa, float3 colour)
{
	const float2 dirs[3] = { float2(0.9659, 0.2588), float2(-0.2588, 0.9659), float2(-0.7071, -0.7071) };
	const float len = 0.05;
	float2 v = x - s;
	float sum = 0;
	[unroll] for (int d = 0; d < 3; d++)
	{
		float along = abs(dot(v, dirs[d])) - r;	// past the limb, either way along the line
		if (along <= 0)
			continue;
		float across = abs(v.x * dirs[d].y - v.y * dirs[d].x);
		float w = max(0.15 * r, 0.25 * aa) * exp(-along / (1.5 * len)) + 0.1 * aa;	// thin at the limb, to a point
		sum += exp(-across * across / (w * w)) * exp(-along / len);
	}
	return sum * colour;
}

// A hash of a 2D cell, 0..1 (Dave Hoskins' hash12, MIT).
float Hash12(float2 p)
{
	float3 p3 = frac(float3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return frac((p3.x + p3.y) * p3.z);
}

// The dirt on the lens, fixed to the screen (the same on every frame): a faint film all over (x) and the marks (y).
// Round 10 (2026-11-26; the owner: every mark a soft round blob of the same size): a mix, as a used lens has it,
// on cell grids of four scales, each cell holding at most one mark at a random place, size and density: many fine
// specks of dust, fewer smudges (dried drops, a little denser at their rim), a few large faint smears, and now and
// then a short streak (a wipe), long and thin at a random angle. EDCAS's own.
float DirtMark(float2 q, float2 c, float density, float radiusLo, float radiusHi, int kind)
{
	float2 centre = c + float2(Hash12(c + 3.1), Hash12(c + 7.7));
	float radius = lerp(radiusLo, radiusHi, Hash12(c + 11.3));
	float2 d = q - centre;
	if (kind == 3)
	{
		// a streak: some seven times as long as wide
		float a = Hash12(c + 13.7) * 3.1415927;
		float2 u = float2(cos(a), sin(a));
		d = float2(dot(d, u), d.x * u.y - d.y * u.x) * float2(1, 7);
	}
	float r = length(d) / radius;
	float mark = smoothstep(1.0, kind == 0 ? 0.3 : 0.1, r);
	if (kind == 1)
		mark *= lerp(0.7, 1.0, smoothstep(0.4, 0.85, r));	// a dried drop's rim
	return mark * density;
}

float2 Dirt(float2 uv, float aspect)
{
	float2 p = uv * float2(aspect, 1);
	float marks = 0;
	// per kind: cells across a picture height, the share of cells with a mark, the radius (of a cell), the density
	const float scales[4] = { 48.0, 14.0, 5.0, 7.0 };
	const float share[4] = { 0.35, 0.40, 0.30, 0.06 };
	const float2 radii[4] = { float2(0.12, 0.35), float2(0.3, 0.85), float2(0.5, 0.95), float2(0.35, 0.8) };
	const float2 dens[4] = { float2(0.15, 0.45), float2(0.12, 0.35), float2(0.05, 0.14), float2(0.12, 0.25) };
	[unroll] for (int k = 0; k < 4; k++)
	{
		float2 q = p * scales[k] + k * 17.0;
		float2 cell = floor(q);
		[unroll] for (int y = -1; y <= 1; y++)
			[unroll] for (int x = -1; x <= 1; x++)
			{
				float2 c = cell + float2(x, y);
				if (Hash12(c) > share[k])	// most cells are clean
					continue;
				marks += DirtMark(q, c, lerp(dens[k].x, dens[k].y, Hash12(c + 5.9)), radii[k].x, radii[k].y, k);
			}
	}
	return float2(0.04, marks);
}

#if defined(FLARE_GRID)
// The star light's sums over a 16 x 8 grid of cells of the glow before its blur (the 32nd of the picture), two
// texels a cell: its colour summed (rgb) and its luminance (a); the luminance times the place (xy, picture
// heights from the centre) and times the place's square (z). Each sum over the whole 32nd's texel count, so that
// FLARE_SUM's totals are means over the picture.
float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
	int2 at = int2(uv * float2(32, 8));
	int2 cell = int2(at.x >> 1, at.y);
	float w, h;
	Glow.GetDimensions(w, h);
	int2 lo = int2(cell * float2(w, h) / float2(16, 8)), hi = int2((cell + 1) * float2(w, h) / float2(16, 8));
	float aspect = PictureSize().x * PictureSize().w;
	float4 a = 0, b = 0;
	[loop] for (int y = lo.y; y < hi.y; y++)
		[loop] for (int x = lo.x; x < hi.x; x++)
		{
			float3 c = Glow.Load(int3(x, y, 0)).rgb;
			float lum = dot(c, float3(0.2126, 0.7152, 0.0722));
			float2 p = ((float2(x, y) + 0.5) / float2(w, h) - 0.5) * float2(aspect, 1);
			a += float4(c, lum);
			b += float4(lum * p, lum * dot(p, p), 0);
		}
	return ((at.x & 1) ? b : a) / (w * h);
}
#elif defined(FLARE_SUM)
// The grid's cells summed (FLARE_GRID): texel 0 the star light's mean colour over the picture, texel 1 its
// light-weighted centre and spread (LensFlare above). The spread less the 32nd's own texel (a uniform square's
// variance, its side squared over 12).

float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
	float4 a = 0, b = 0;
	[loop] for (int y = 0; y < 8; y++)
		[loop] for (int x = 0; x < 16; x++)
		{
			a += FlareGrid.Load(int3(2 * x, y, 0));
			b += FlareGrid.Load(int3(2 * x + 1, y, 0));
		}
	if (uv.x < 0.5)
		return float4(a.rgb, 1);
	if (a.a <= 1e-9)
		return 0;
	float w, h;
	Glow.GetDimensions(w, h);
	float2 centre = b.xy / a.a;
	float texel = 1 / h;	// the 32nd's texel in picture heights
	float variance = max((b.z / a.a - dot(centre, centre)) / 2 - texel * texel / 12, 0);
	return float4(centre, sqrt(variance), 1);
}
#elif defined(DIRT_H) || defined(DIRT_V)
// The glow blurred across (DIRT_H) or down (DIRT_V): 25 taps of a Gaussian of sigma 4 texels.
float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
	float w, h;
	Glow.GetDimensions(w, h);
#if defined(DIRT_H)
	float2 step = float2(1 / w, 0);
#else
	float2 step = float2(0, 1 / h);
#endif
	float3 sum = 0;
	float total = 0;
	[unroll] for (int i = -12; i <= 12; i++)
	{
		float g = exp(-i * i / 32.0);
		sum += g * BilinearAt(Glow, uv + step * i).rgb;
		total += g;
	}
	return float4(sum / total, 1);
}
#elif defined(DIRT_LIGHT)
// The glow before its blur: each texel the mean star light over the 32 x 32 pixels it covers, 8 x 8 reads.
float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
	float2 texel = 32 * PictureSize().zw;
	float3 sum = 0;
	[unroll] for (int y = 0; y < 8; y++)
		[unroll] for (int x = 0; x < 8; x++)
			sum += BrightAt(uv + (float2(x, y) - 3.5) / 8 * texel);
	return float4(sum / 64, 1);
}
#else
// Each effect's share of the star's light at strength 1 (tuned on ed_frame27784, a sun filling a fifth of the
// picture, and on the same sun made small, 2026-11-26).
#ifndef GHOST_GAIN
#define GHOST_GAIN 2.5
#endif
#ifndef STREAK_GAIN
#define STREAK_GAIN 0.15
#endif
#ifndef SPIKE_GAIN
#define SPIKE_GAIN 0.08
#endif
#ifndef DISC_SPIKE
#define DISC_SPIKE 0.8	// a small disc's ray at the limb, against the sampled rays' light
#endif
#ifndef DIRT_GAIN
#define DIRT_GAIN 0.3
#endif

float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
	float4 size = PictureSize();	// the picture's (t110), for its shape
	float4 r609 = PostRow(609), r611 = PostRow(611);
	float2 step = 2 * size.zw;	// two pixels
	float3 lens = 0;
	if (r609.x > 0)
	{
		// a ghost is a ten-thousandth of its star or less: over the star's own picture it is lost (the picture
		// holds the disc near white, so a ghost added there would lighten it, a hot spot where they gather)
		float3 star = BrightAt(uv);
		float over = saturate(max(star.r, max(star.g, star.b)));
		lens += LensFlare(uv, size.x * size.w, 6 * size.w, saturate(r611.x)) * (1 - over) * GHOST_GAIN * r609.x;
	}
	if (r609.y > 0)
		lens += Streaks(uv, step) * STREAK_GAIN * r609.y;
	if (r611.x > 0)
	{
		// round 11 (0.43.296): fading out as the visible star grows (a wide disc smears its spikes into an even glow
		// that Elite's bloom already gives: a near-white disc covering 0.6 % of the picture keeps 60 %, 1 % some 30 %,
		// 10 % nothing to see), and never on the star's own picture (they veiled its surface)
		float4 shape = Flare.Load(int3(1, 0, 0));
		float area = dot(Flare.Load(int3(0, 0, 0)).rgb, float3(0.2126, 0.7152, 0.0722)) / 0.006;
		float3 star = BrightAt(uv);
		float over = saturate(max(star.r, max(star.g, star.b)));
		// a point-like star keeps the sampled rays (Spikes); a small disc's are drawn from its centre (DiscSpikes)
		float3 total = Flare.Load(int3(0, 0, 0)).rgb;
		float3 colour = total / max(max(total.r, max(total.g, total.b)), 1e-6);
		float disc = smoothstep(0.005, 0.012, shape.z);
		float3 spikes = lerp(Spikes(uv, step, size.x * size.w, shape.z),
			DiscSpikes((uv - 0.5) * float2(size.x * size.w, 1), shape.xy, 2 * shape.z, 6 * size.w, colour) * DISC_SPIKE, disc);
		lens += spikes * (1 - over) / (1 + area * area) * SPIKE_GAIN * r611.x;
	}
	if (r611.y > 0)
	{
		// round 10: the marks lit by the star's glow in the star's colour with the ghosts' warm lean (grey-green
		// stains on an orange glow in the game), and a little stronger the larger the visible star (a screen-filling
		// sun's glow reaches only a thin band past its long limb: its dirt all but vanished at strength 1; up to
		// 1.5 times from 2 % to 15 % of the picture); the film keeps its light, so the veil round the rim does not
		// grow
		float3 glow = Compress(GlowAt(uv));
		float3 light = Flare.Load(int3(0, 0, 0)).rgb;
		float area = dot(light, float3(0.2126, 0.7152, 0.0722));
		float3 star = light / max(max(light.r, max(light.g, light.b)), 1e-6);
		float3 warm = lerp(1, GHOST_WARM, saturate((star.r - star.b) / 0.4));
		float2 dirt = Dirt(uv, size.x * size.w);
		lens += glow * warm * (dirt.x + dirt.y * (1 + 0.5 * smoothstep(0.02, 0.15, area))) * DIRT_GAIN * r611.y;
	}
	return float4(lens, 1);
}
#endif
#endif
