// The post effects' lens pass (edcas_post.hlsl), last: the lens flare, anamorphic streaks, diffraction spikes and
// lens dirt (their light from
// edcas_post_lenslight.hlsl), chromatic aberration, the vignette, film grain, and the
// colour-blind correction over everything (SweetFX ChromaticAberration.fx, Vignette.fx and FilmGrain.fx at their
// defaults, the rows scaling the aberration's blend, the vignette's amount and the grain's intensity, 1 SweetFX's;
// Daltonize.fx, daltonize.org's LMS daltonization, blended in by its row).
#include "edcas_post.hlsl"

// The lens light (lens flare, anamorphic streaks, diffraction spikes, lens dirt), drawn at a quarter of the picture's size by
// edcas_post_lenslight.hlsl (edcas.ini ResourceEdcasPostLensLight), in linear light.
Texture2D<float4> LensLight : register(t114);

// Lateral chromatic aberration as a lens has it (EDCAS's own, 2026-11-26): the red picture a little larger than
// the green, the blue a little smaller, so the shift is nothing at the centre and grows with the distance from it,
// along the line from the centre: at strength 1 about 5 px at the left and right borders of a 3440 px wide
// picture and 2 px at the top and bottom of one 1440 px high; the row's top is 2. SweetFX's ChromaticAberration.fx, used before, shifted the whole
// picture by one fixed (2.5, -0.5) px and blended that in by half: fringes in the middle of the picture, which no
// lens makes, and nothing more to gain above strength 2 (the owner's screenshots, README.md "Post effects").
float3 Aberration(float3 c, float2 uv, float2 px, float strength)
{
	const float scale = 0.003;	// the red picture's magnification less 1, at strength 1
	float2 d = (uv - 0.5) * scale * strength;
	return float3(PictureAt(uv - d).r, c.g, PictureAt(uv + d).b);
}

// Film grain: SweetFX FilmGrain.fx's monochrome Gaussian noise, times the colour, with EDCAS's own weighting by
// brightness (2026-11-26). SweetFX's (1 - brightness)^6 gives a mid-grey 1.6 % of the grain and a bright pixel
// none: in Elite the owner saw grain on a dark planet alone and none on its bright ring, even at strength 3. Film's
// grain shows most in the mid-tones and fades towards black (no silver) and white (all silver): here
// 4 l (1 - l) on the display-encoded brightness, with a floor of 0.25 towards white so that a bright surface keeps
// a little; black keeps none (the noise multiplies the colour).
float3 Grain(float3 c, float2 uv, float seconds, float strength)
{
	const float variance = 0.40, mean = 0.5;
	float intensity = 0.08 * strength;
	float luma = saturate((c.r + c.g + c.b) / 3);
	float weight = max(4 * luma * (1 - luma), 0.25 * luma);
	float t = frac(seconds * 0.0022337 * 1000 / 64) * 64;	// SweetFX's timer is milliseconds; kept small for float
	float seed = dot(uv, float2(12.9898, 78.233));
	float u1 = frac(sin(seed) * 43758.5453 + t);
	float u2 = frac(cos(seed) * 53758.5453 - t);
	float v = variance * weight;
	float r = sqrt(-log(max(u1, 1e-4)));
	float g = v * r * cos(6.28318531 * u2) + mean;
	return c * lerp(1 + intensity, 1 - intensity, g);
}

// SweetFX Vignette.fx type Original: ratio 1, radius 2, amount -1 (times the strength), slope 2, centred.
float3 Vignette(float3 c, float2 uv, float aspect, float strength)
{
	float2 d = (uv - 0.5) * float2(aspect, 1.0) / 2.0;
	float dist = dot(d, d);
	return c * (1.0 + pow(dist, 2 * 0.5) * -1.0 * strength);
}

// Daltonize.fx (daltonize.org): what a colour-blind eye of the type (0 protanopia, 1 deuteranopia, 2 tritanopia)
// loses, simulated in LMS, shifted into the channels it can tell apart.
float3 Daltonize(float3 c, int type)
{
	float L = 17.8824 * c.r + 43.5161 * c.g + 4.11935 * c.b;
	float M = 3.45565 * c.r + 27.1554 * c.g + 3.86714 * c.b;
	float S = 0.0299566 * c.r + 0.184309 * c.g + 1.46709 * c.b;
	float l = L, m = M, s = S;
	if (type == 0)
		l = 2.02344 * M - 2.52581 * S;
	else if (type == 1)
		m = 0.494207 * L + 1.24827 * S;
	float3 seen;
	seen.r = 0.0809444479 * l - 0.130504409 * m + 0.116721066 * s;
	seen.g = -0.0102485335 * l + 0.0540193266 * m - 0.113614708 * s;
	seen.b = -0.000365296938 * l - 0.00412161469 * m + 0.693511405 * s;
	// Tritanopia by Machado, Oliveira and Fernandes' matrix (2009, severity 1). The LMS row copied everywhere,
	// s = -0.395913 L + 0.801109 M, gives a negative blue for an orange (-0.6 for (0.8, 0.5, 0.2)): no eye sees that,
	// and the correction built on it changed a whole orange picture (2026-11-26).
	if (type == 2)
		seen = float3(dot(c, float3(1.255528, -0.076749, -0.178779)), dot(c, float3(-0.078411, 0.930809, 0.147602)),
		              dot(c, float3(0.004733, 0.691367, 0.303900)));
	// What the eye lost goes into the channels it can tell apart. Protanopia and deuteranopia share Fidaner, Lin and
	// Ozguven's rule (the lost red-green difference into green and blue); tritanopia has its own (the lost blue into
	// red and green). Until 2026-11-26 the first rule stood for all three: for tritanopia it put the lost blue back
	// onto blue, the one channel that eye cannot see, and the whole picture turned violet (the owner's screenshot).
	// Measured by daltonize_check.py on a chart of all hues, through Machado's model of each eye: the pairs an eye
	// confuses are seen 2.3 (protanopia), 1.6 (deuteranopia) and 2.1 (tritanopia) times further apart. A rule of
	// its own for deuteranopia (the loss into red and blue) gave 1.1 there and was dropped.
	float3 err = c - seen;
	float3 corr = type == 2 ? float3(err.r + 0.7 * err.b, err.g + 0.7 * err.b, 0)
	                        : float3(0, 0.7 * err.r + err.g, 0.7 * err.r + err.b);
	return c + corr;
}

float4 main(float4 pos : SV_Position) : SV_Target0
{
	float4 size = PictureSize();
	float2 uv = pos.xy * size.zw;
	float4 picture = Picture.Load(int3(pos.xy, 0));
	float3 c = picture.rgb;
	float4 r600 = PostRow(600), r601 = PostRow(601), r609 = PostRow(609), r611 = PostRow(611);
	if (r609.x > 0 || r609.y > 0 || r611.x > 0 || r611.y > 0)
	{
		// light from the lens adds to the scene's light; dithered by up to half an 8-bit step where it adds any
		// (the picture is written in 8 bits: a large faint ghost, some 15 steps over 400 pixels, showed its steps
		// as rings, client 0.43.290)
		float3 lens = BilinearAt(LensLight, uv).rgb;
		float3 before = c;
		c = ToSrgb(ToLinear(c) + lens);
		if (any(lens > 0))
		{
			float2 p = pos.xy;
			float n = frac(52.9829189 * frac(dot(p, float2(0.06711056, 0.00583715))));	// interleaved gradient noise (Jimenez 2014)
			float m = frac(52.9829189 * frac(dot(p + 17.3, float2(0.06711056, 0.00583715))));
			c = max(c + (n + m - 1) * 0.5 / 255, before);	// triangular, within half a step; never below the picture: the lens only adds light
		}
	}
	if (r600.x > 0)
		c = saturate(Aberration(c, uv, size.zw, r600.x));	// its top is 2 (client post_effects.rs)
	float4 r603 = PostRow(603), r607 = PostRow(607);
	if (r603.x > 0)
		c = saturate(Vignette(c, uv, size.x * size.w, r603.x));
	if (r601.y > 0)
		c = saturate(Grain(c, uv, r601.w, r601.y));
	if (r607.x > 0)
		c = lerp(c, Daltonize(c, (int)round(r607.y)), saturate(r607.x));	// the type from $daltonize_type, unfaded (edcas.ini)
	return float4(saturate(c), picture.a);
}
