// Elite's skybox near a black hole, with the hole's real optics (edcas.ini [ShaderOverrideEdcasBlackHole],
// README.md "Black holes", 2026-11-25). Drawn instead of Elite's skybox aa79b1b7c92b01ac (handling =
// skip, draw = from_caller: Elite's vertex shader, constants and cubemaps), which Elite uses in every
// frame of a system with a black hole (README-ELITE.md, "Black holes").
//
// Elite's own lens is a smooth squeeze of the sky towards the hole with a dark ramp r/R0 in the middle
// (black only at the centre) and its "Einstein ring" on the ramp's edge. Here every pixel's ray is bent
// as a photon in the Schwarzschild metric bends, for a camera at rest at distance r:
//   - the shadow: rays with impact parameter b < (3 sqrt(3) / 2) r_s end in the hole: 0, no light at all;
//   - outside it the ray's direction at infinity (edcas_lens.hlsl: Darwin's exact deflection, checked against
//     an integration of the photon's orbit).
//   The Einstein ring, the second image on the far side and the photon ring at the shadow's edge come out
//   of the mapping; nothing is drawn for them.
// Elite's constants (rows of its cb2): [18].xyz the hole's direction, [21].x = r_s / r (k = 1.000: 0.9991 at
// 950 km, 1.0006 at 5 Mm by the HUD, ed_frame37303 and ed_frame81414), [15], [16], [17] its sky's colour and
// tone. The sky's colour per direction and the tone are Elite's own (`skyColour`, `tone`).
//
// The accretion disc, where the client says there is one (IniParams 551, 552, the blackbody table 560 to 591;
// crates/client/src/black_hole.rs decides from the journal): a thin, opaque Shakura-Sunyaev disc from the
// innermost stable orbit (3 r_s) out to z551, T(R) = T_s (R / r_s)^(-3/4) (1 - sqrt(3 r_s / R))^(1/4). Each pixel's
// photon is followed back along its orbit (Binet's u'' = -u + 3/2 r_s u^2, RK4 in 24 steps a half turn) to where it
// crosses the disc's plane: the first crossing is the near side, the next ones the far side bent over the top and
// under the bottom of the shadow and the disc's ring images. The light there is a blackbody at g T(R), with
//   g = sqrt(1 - 3 r_s / 2R) / (sqrt(1 - r_s / r) (1 - Omega lambda)),
// Omega = sqrt(r_s / 2R^3) c the gas's circular orbit, lambda the photon's angular momentum about the disc's axis
// over its energy: the gravitational redshift, the Doppler shift of the orbiting gas (the approaching side bluer
// and brighter: a blackbody's radiance at g T is the g^3 boost of I_nu) and the camera's blueshift, at rest
// (Luminet 1979). Its brightness is Elite's own: DISC_SUN_SURFACE, the scene's value for a surface as bright
// as the Sun's.
//
// Not modelled: the camera's motion (aberration), the gravitational blueshift of the sky's light at the
// camera (1 / (1 - r_s / r): 1.5 % in frequency at 950 km), the disc's thickness and limb darkening, the
// hole's spin.

cbuffer cb2 : register(b2)
{
	float4 cb2[25];
}

TextureCube<float4> t0 : register(t0);
TextureCube<float4> t1 : register(t1);
SamplerState s0 : register(s0);
Texture1D<float4> IniParams : register(t120);

#include "edcas_lens.hlsl"

float4 iniRow(int row)
{
	return IniParams.Load(int2(row, 0));
}

// The scene's value of a surface as bright as the Sun's (5778 K), in Elite's units: what the table's 1 is.
static const float DISC_SUN_SURFACE = 0.315;
// The blackbody table (client black_hole.rs): linear sRGB radiance at 10^(3 + i 5/31) K, the Sun's surface 1.
static const int DISC_TABLE = 560;
static const int DISC_TABLE_ROWS = 32;

// A blackbody's linear sRGB radiance at T kelvin, the Sun's surface luminance 1: the table, log-interpolated;
// past 10^8 K Rayleigh-Jeans (radiance as T), under 1000 K the table's first step carried on.
float3 blackbody(float T)
{
	if (T <= 100)
		return 0;
	float l = (log10(T) - 3) * (31.0 / 5);
	if (l >= DISC_TABLE_ROWS - 1)
		return iniRow(DISC_TABLE + DISC_TABLE_ROWS - 1).xyz * (T / 1e8);
	int i = clamp((int)floor(l), 0, DISC_TABLE_ROWS - 2);
	float f = l - i;
	float3 a = log2(max(iniRow(DISC_TABLE + i).xyz, 1e-30)), b = log2(max(iniRow(DISC_TABLE + i + 1).xyz, 1e-30));
	return exp2(lerp(a, b, f));
}

// Binet's equation for a photon, u = r_s / r against the angle phi swept: (u, u').
float2 binet(float2 s)
{
	return float2(s.y, -s.x + 1.5 * s.x * s.x);
}

// Where the photon of a pixel crosses the disc, followed back from the camera: the disc's radius R (r_s) and its
// g. v the pixel's direction, H the hole's, x = r_s / r, n the disc's axis, outer its edge (r_s).
bool discHit(float3 v, float3 H, float x, float3 n, float outer, out float R, out float g)
{
	R = 0;
	g = 0;
	float ct = dot(v, H);
	float3 e = v - H * ct;
	float st = length(e);
	if (st < 1e-6)
		return false;	// straight at the hole or away from it: along the disc's axis only by chance
	e /= st;
	float3 c = -H;	// from the hole to the camera
	float B = st / (x * sqrt(1 - x));	// impact parameter / r_s
	// the photon's place at phi: c cos(phi) + e sin(phi); it crosses the plane where that is at right angles to n
	float phi1 = atan2(-dot(n, c), dot(n, e));
	if (phi1 <= 0)
		phi1 += LENS_PI;
	float2 s = float2(x, sqrt(max(1 / (B * B) - x * x * (1 - x), 0)) * (ct > 0 ? 1 : -1));
	float phi = 0;
	const float h0 = LENS_PI / 24;
	[loop] for (int k = 0; k < 3; k++)
	{
		float to = phi1 + k * LENS_PI;
		int steps = max((int)ceil((to - phi) / h0), 1);
		float h = (to - phi) / steps;
		[loop] for (int i = 0; i < steps; i++)
		{
			float2 k1 = binet(s), k2 = binet(s + 0.5 * h * k1), k3 = binet(s + 0.5 * h * k2), k4 = binet(s + h * k3);
			s += h / 6 * (k1 + 2 * k2 + 2 * k3 + k4);
			if (s.x <= 0 || s.x >= 1)
				return false;	// gone off to infinity, or into the hole, before the plane
		}
		phi = to;
		R = 1 / s.x;
		if (R >= 3 && R <= outer)
		{
			// lambda: the photon's angular momentum about n over its energy, b times the cosine; the photon runs
			// the other way from the traced ray, whose orbit turns about c x e.
			float lambda = -B * dot(cross(c, e), n);
			float omega = sqrt(0.5 / (R * R * R));
			g = sqrt(1 - 1.5 / R) / (sqrt(1 - x) * (1 - omega * lambda));
			return true;
		}
	}
	return false;
}

// Elite's sky colour in a direction (its two cubemaps' product), before its exposure and tone.
float3 skyCube(float3 d)
{
	return t1.Sample(s0, d).xyz * float3(3.24044061, 1.64095843, 12.195122) * t0.Sample(s0, d).x * 3;
}

// Elite's tone of the sky (its skybox's branch on cb2[17].x, unchanged): c the cubemap colour of the
// pixel's image, col the colour so far, w = TEXCOORD3.x.
float3 tone(float3 c, float3 col, float w)
{
	if (asuint(cb2[17].x) == 0)
		return col;
	const float3 lum = float3(0.2125, 0.7154, 0.0721);
	float w0 = cb2[17].y + 1;
	float a = saturate(w * 0.0833333358);
	float hi = asuint(cb2[16].w) != 0 ? a * 8 - 7 : cb2[17].z;
	float ex = w <= 12 ? a * -0.875 + 1 : saturate((w - 12) * 0.166666672) * -0.09375 + 0.125;
	ex = asuint(cb2[16].w) != 0 ? ex : cb2[17].w;
	float L = exp2(log2(dot(exp2(log2(c) * 0.15), lum)) * ex);
	w0 = L * (hi - w0) + w0;
	float Y = max(dot(col, lum), 9.999999939e-09);
	w0 += log2(Y * 8);
	float3 o = ((Y - col) * 0.15 + col) * float3(0.7, 0.855, 0.855);
	return exp2(w0 - 3) * o / Y;
}

void main(float4 v0 : TEXCOORD1, float v1 : TEXCOORD3, out float4 o0 : SV_Target0)
{
	float3 v = normalize(v0.xyz);
	float3 dir;
	o0 = float4(0, 0, 0, 1);
	float4 disc = iniRow(551);
	float R, g;
	float3 glow = 0;
	if (disc.x > 0.5 && cb2[21].x > 0 && discHit(v, cb2[18].xyz, cb2[21].x, normalize(iniRow(552).xyz), disc.z, R, g))
	{
		float3 light = blackbody(g * disc.y * pow(max(R, 3), -0.75) * pow(saturate(1 - sqrt(3 / max(R, 3))), 0.25)) * DISC_SUN_SURFACE;
		// A thin disc (kind 1, fed by a companion) is opaque: it hides what lies behind it. Sagittarius A*'s flow (kind 3) is hot,
		// thick and optically thin: its light is added to whatever lies behind it, the lensed sky or the shadow, and
		// it hides nothing. A blackbody at its temperature is the most it can give (an optically thin gas emits less
		// than a blackbody at its temperature): an upper bound.
		if (disc.w < 2.5)
		{
			o0.xyz = light;
			return;
		}
		glow = light;
	}
	o0.xyz = glow;
	if (!lensDir(v, cb2[18].xyz, cb2[21].x, dir))
		return;	// the shadow: nothing comes out of it (but a thin flow in front of it)
	float3 c = skyCube(dir);
	float3 col = exp2(log2(c) * cb2[16].x) * cb2[15].xyz * cb2[15].w;
	o0.xyz = tone(c, col, v1) * cb2[16].y + glow;
}
