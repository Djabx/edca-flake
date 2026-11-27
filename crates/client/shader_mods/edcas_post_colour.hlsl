// The post effects' colour pass (edcas_post.hlsl): clarity, fake HDR, levels, white balance, lift/gamma/gain, the DPX film
// look, Technicolor, bleach bypass, contrast curves, vibrance, monochrome, sepia, in that order (SweetFX's formulas, each at its default and blended in by its row: 1 is SweetFX's look, above 1
// goes further on the same line). Only fake HDR reads neighbours, so they share one pass.
#include "edcas_post.hlsl"

// Clarity's surroundings: the brightness blurred over some 12 pixels at a quarter of the picture's size
// (edcas_post_clarity.hlsl).
Texture2D<float4> ClaritySurround : register(t115);

// Clarity: the pixel's brightness against its surroundings, raised, most in the mid-tones (shadows and highlights
// keep their detail as it is, so nothing clips or turns muddy). The difference is held to +-0.15, which keeps the
// halo round a bright edge (a planet's limb against space) faint. The colour is scaled to its new brightness, so
// it keeps its saturation (added to all three channels alike, the change greyed the colours, ed_frame171605).
float3 Clarity(float3 c, float2 uv, float strength)
{
	float l = dot(c, float3(0.2126, 0.7152, 0.0722));
	float around = BilinearAt(ClaritySurround, uv).r;
	float detail = clamp(l - around, -0.15, 0.15);
	float mid = 1 - pow(2 * l - 1, 4);
	float want = l + detail * mid * 0.6 * strength;
	return c * clamp(want / max(l, 1e-3), 0.5, 2.0);
}

// SweetFX FakeHDR.fx: two rings of eight taps, a local contrast step and a power.
float3 FakeHdr(float3 c, float2 uv, float2 px)
{
	const float power = 1.30, radius1 = 0.793, radius2 = 0.87;
	const float2 ring[8] = { float2(1.5, -1.5), float2(-1.5, -1.5), float2(1.5, 1.5), float2(-1.5, 1.5),
		float2(0, -2.5), float2(0, 2.5), float2(-2.5, 0), float2(2.5, 0) };
	float3 b1 = 0, b2 = 0;
	[unroll] for (int i = 0; i < 8; i++)
	{
		b1 += PictureAt(uv + ring[i] * radius1 * px);
		b2 += PictureAt(uv + ring[i] * radius2 * px);
	}
	b1 *= 0.005;
	b2 *= 0.010;
	float3 hdr = (c + (b2 - b1)) * (radius2 - radius1);
	return saturate(pow(abs(c + hdr), power) + hdr);
}

// SweetFX DPX.fx (Loadus): a logistic S-curve per channel, colourfulness, saturation in its own XYZ-like space.
float3 Dpx(float3 c)
{
	const float3 curve = float3(8, 8, 8), centre = float3(0.36, 0.36, 0.34);
	const float contrast = 0.1, saturation = 3.0, colourfulness = 2.5;
	const float3x3 toXyz = float3x3(
		0.5003033835433160, 0.3380975732227390, 0.1645897795458570,
		0.2579688942747580, 0.6761952591447060, 0.0658358459823868,
		0.0234517888692628, 0.1126992737203000, 0.8668396731242010);
	const float3x3 toRgb = float3x3(
		2.6714711726599600, -1.2672360578624100, -0.4109956021722270,
		-1.0251070293466400, 1.9840911624108900, 0.0439502493584124,
		0.0610009456429445, -0.2236707508128630, 1.1590210416706100);
	float3 b = c * (1 - contrast) + 0.5 * contrast;
	float3 t = 1 / (1 + exp(curve / 2));
	b = (1 / (1 + exp(-curve * (b - centre)))) / (1 - 2 * t) - t / (1 - 2 * t);
	float v = max(max(b.r, b.g), max(b.b, 1e-6));
	float3 c0 = pow(abs(b / v), 1 / colourfulness) * v;
	c0 = mul(toXyz, c0);
	float l = dot(c0, float3(0.30, 0.59, 0.11));
	c0 = (1 - saturation) * l + saturation * c0;
	return mul(toRgb, c0);
}

// SweetFX Curves.fx: the luma through the simplified Catmull-Rom S-curve (formula 4), the chroma kept. The blend
// is 1.0 at strength 1 (SweetFX's 0.65 doubled, 1.3, as it stood until 2026-11-26, went past the curve) and eased
// to 0.6 of that towards black: most of Elite's picture is dark, and the curve's lower half took a dark planet's
// plains to black (the owner's screenshots). The row's top is 1.5.
float3 Curves(float3 c, float strength)
{
	float l = dot(c, float3(0.2126, 0.7152, 0.0722));
	float3 chroma = c - l;
	float x = saturate(l);
	x = x * (x * (1.5 - x) + 0.5);
	return lerp(l, x, strength * lerp(0.6, 1.0, saturate(2 * l))) + chroma;
}

// SweetFX Vibrance.fx: saturation raised the more, the less saturated a pixel is.
float3 Vibrance(float3 c, float strength)
{
	float l = dot(c, float3(0.212656, 0.715158, 0.072186));
	float sat = max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
	float k = 0.30 * strength;	// SweetFX's 0.15 was hard to see in the game (2026-11-26); the row's top is 2
	return lerp(l, c, 1 + k * (1 - sign(k) * sat));
}

// SweetFX Levels.fx: a black point and a white point stretched to 0 and 255; the strength moves the points from
// 0 and 255 towards them. The white point is SweetFX's 235; the black point is 4, not SweetFX's 16 (made for a
// TV-range picture): Elite's picture uses the full range, and 26 to 58 % of a frame lies under 16 (5 to 47 % under
// 4; the tonemap pictures of ed-captures/post/tone-*, 2026-11-26), so 16 only cut shadow detail. The row's top is 1.5.
float3 Levels(float3 c, float strength)
{
	float black = 4.0 / 255.0 * strength, white = 1.0 - 20.0 / 255.0 * strength;
	return (c - black) / max(white - black, 1e-3);
}

// SweetFX LiftGammaGain.fx (3an, CeeJay.dk): lift, then gain, then gamma, each per channel, 1 neutral.
float3 LiftGammaGain(float3 c, float3 lift, float3 gamma, float3 gain)
{
	c = c * (1.5 - 0.5 * lift) + 0.5 * lift - 0.5;
	c = saturate(c) * gain;
	return saturate(pow(abs(c), 1.0 / max(gamma, 0.01)));
}

// SweetFX Technicolor.fx (DKT70, CeeJay.dk): the three-strip process's negatives, power 4, negative amount 0.88,
// blended in by 0.4 times the strength.
float3 Technicolor(float3 c, float strength)
{
	const float power = 4.0, negative = 0.88;
	const float3 cyan = float3(0.0, 1.30, 1.0), magenta = float3(1.0, 0.0, 1.05), yellow = float3(1.6, 1.6, 0.05);
	const float2 redOrange = float2(1.05, 0.620), green = float2(0.30, 1.0), magenta2 = float2(1.0, 1.05);
	float2 nr = c.rg / (negative * power), ng = c.rg / (negative * power), nb = c.rb / (negative * power);
	float3 r = dot(redOrange, nr) + cyan;
	float3 g = dot(green, ng) + magenta;
	float3 b = dot(magenta2, nb) + yellow;
	return lerp(c, r * g * b, 0.4 * strength);
}

// A black body's colour at `kelvin` (Tanner Helland's fit to the CIE 1964 10-degree data, sRGB-encoded 0..1).
float3 Kelvin(float kelvin)
{
	float t = kelvin / 100;
	float r = t <= 66 ? 255 : 329.698727446 * pow(max(t - 60, 1e-3), -0.1332047592);
	float g = t <= 66 ? 99.4708025861 * log(t) - 161.1195681661 : 288.1221695283 * pow(max(t - 60, 1e-3), -0.0755148492);
	float b = t >= 66 ? 255 : (t <= 19 ? 0 : 138.5177312231 * log(max(t - 10, 1e-3)) - 305.0447927307);
	return saturate(float3(r, g, b) / 255);
}

// White balance: the picture as if lit by a black body of `kelvin` instead of daylight (6500 K), in linear light,
// its brightness kept (EDCAS's own).
float3 WhiteBalance(float3 c, float kelvin)
{
	float3 w = ToLinear(Kelvin(kelvin)) / max(ToLinear(Kelvin(6500)), 1e-4);
	w /= dot(w, float3(0.2126, 0.7152, 0.0722));
	return ToSrgb(ToLinear(c) * w);
}

// Bleach bypass (the film process that skips the bleach and keeps the silver: harsh contrast, muted colour), as
// NVIDIA's shader library has it: an overlay of the picture with its own luma; 0.6 times the strength.
float3 BleachBypass(float3 c, float strength)
{
	float lum = dot(c, float3(0.25, 0.65, 0.1));
	float3 low = 2 * c * lum, high = 1 - 2 * (1 - lum) * (1 - c);
	float3 bleached = lerp(low, high, saturate(10 * (lum - 0.45)));
	return lerp(c, bleached, 0.6 * strength);
}

// Monochrome (SweetFX Monochrome.fx's first weights), blended in by the strength.
float3 Monochrome(float3 c, float strength)
{
	return lerp(c, dot(c, float3(0.21, 0.72, 0.07)), saturate(strength));
}

// Sepia: the common sepia-tone matrix, blended in by the strength.
float3 Sepia(float3 c, float strength)
{
	float3 s = float3(dot(c, float3(0.393, 0.769, 0.189)), dot(c, float3(0.349, 0.686, 0.168)), dot(c, float3(0.272, 0.534, 0.131)));
	return lerp(c, s, saturate(strength));
}

float4 main(float4 pos : SV_Position) : SV_Target0
{
	float4 size = PictureSize();
	float2 uv = pos.xy * size.zw;
	float4 picture = Picture.Load(int3(pos.xy, 0));
	float3 c = picture.rgb;
	float4 r600 = PostRow(600), r601 = PostRow(601), r603 = PostRow(603), r609 = PostRow(609), r610 = PostRow(610);
	float4 r611 = PostRow(611);
	// Each effect's result is clamped to 0..1 before the next, as ReShade's 8-bit buffer between two effects clamps
	// it (code review 2026-11-26: Levels gave 1.07 for a bright pixel, and the next effects took it as it was).
	if (r611.z > 0)
		c = saturate(Clarity(c, uv, r611.z));
	if (r600.w > 0)
		c = saturate(lerp(c, FakeHdr(c, uv, size.zw), r600.w));
	if (r603.y > 0)
		c = saturate(Levels(c, r603.y));
	if (r609.z > 0)
		c = saturate(lerp(c, WhiteBalance(c, clamp(r609.w, 1000, 40000)), saturate(r609.z)));
	if (r603.z > 0)
		c = saturate(lerp(c, LiftGammaGain(c, PostRow(604).xyz, PostRow(605).xyz, PostRow(606).xyz), saturate(r603.z)));
	if (r600.y > 0)
		c = saturate(lerp(c, Dpx(c), 0.20 * r600.y));
	if (r603.w > 0)
		c = saturate(Technicolor(c, r603.w));
	if (r610.x > 0)
		c = saturate(BleachBypass(c, r610.x));
	if (r600.z > 0)
		c = saturate(Curves(c, r600.z));
	if (r601.x > 0)
		c = saturate(Vibrance(c, r601.x));
	if (r610.y > 0)
		c = saturate(Monochrome(c, r610.y));
	if (r610.z > 0)
		c = Sepia(c, r610.z);
	return float4(saturate(c), picture.a);
}
