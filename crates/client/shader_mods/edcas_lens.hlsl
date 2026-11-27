// The bending of light by a Schwarzschild black hole, for a camera at rest at distance r from it
// (README.md "Black holes", 2026-11-25). Included by edcas_black_hole.hlsl (the sky) and
// edcas_star_lens.hlsl (the stars, spliced into Elite's star vertex shader).
//
// x = r_s / r (Elite's own constant: cb2[21].x of the skybox, cb1[77].x of the stars). A ray leaving the
// camera at angle theta from the hole's direction has the impact parameter b = r sin(theta) / sqrt(1 - x);
// those with b < (3 sqrt(3) / 2) r_s fall in (the shadow: sin of its radius = (sqrt(27) / 2) x sqrt(1 - x),
// Synge 1966). The others leave for infinity in a direction psi from the hole's (towards the ray's side;
// psi < 0: past the hole, the far side's image), psi = I - alpha:
//   alpha = Darwin's whole deflection from infinity to infinity (exact, an elliptic integral, Carlson's R_F);
//   I = the angle the straight ray would sweep from the camera to infinity, to first order in r_s:
//       t - (r_s / 2b)(sec t + cos t - 2), sin t = b / r.
// Beyond 60 degrees or b > 60 r_s, the series to second order in r_s / b:
//   psi = theta - (1 + cos theta) r_s / b - (15/16)(pi - theta + sin theta cos theta)(r_s / b)^2
// (the third order's term is (16/3)(r_s / b)^3 close to the hole and less further out: 2.5e-5 rad at b = 60 r_s).
// Checked against an integration of the photon's orbit (scratch lens.py, 2026-11-25): Darwin's form to 1e-12;
// the switch at 60 degrees differs by 2e-5 rad for a camera at 67 r_s (950 km from Outords LM-W f1-3), less
// than a tenth of a pixel; 7e-7 rad inside 30 degrees.

#ifndef EDCAS_LENS_HLSL
#define EDCAS_LENS_HLSL

static const float LENS_PI = 3.14159265;
// b / r_s of the rays that circle on the photon sphere: the shadow's edge.
static const float LENS_B_CRIT = 2.59807621;

// Carlson's symmetric elliptic integral R_F(x, y, z): duplication, then the fifth-order series.
float lensRF(float x, float y, float z)
{
	[unroll] for (int i = 0; i < 3; i++)	// in float 3 steps leave 8e-6 rad of deflection (10: 3e-6), a hundredth of a pixel
	{
		float sx = sqrt(x), sy = sqrt(y), sz = sqrt(z);
		float l = sx * sy + sy * sz + sz * sx;
		x = 0.25 * (x + l);
		y = 0.25 * (y + l);
		z = 0.25 * (z + l);
	}
	float m = (x + y + z) / 3;
	float X = 1 - x / m, Y = 1 - y / m, Z = -X - Y;
	float e2 = X * Y - Z * Z, e3 = X * Y * Z;
	return (1 - e2 / 10 + e3 / 14 + e2 * e2 / 24 - 3 * e2 * e3 / 44) / sqrt(m);
}

// The whole deflection of a ray from infinity to infinity, impact parameter B in r_s (B > LENS_B_CRIT):
// Darwin 1959 (Luminet 1979, eq. 3 to 5) with M = 1/2; P the periapsis, from b^2 = P^3 / (P - r_s).
float lensDeflection(float B)
{
	float P = (2 * B / sqrt(3.0)) * cos(acos(-LENS_B_CRIT / B) / 3);
	float Q = sqrt((P - 1) * (P + 3));
	float qp = (2 * P - 3) / (Q + P);	// Q - P without the cancellation
	float k2 = min((qp + 3) / (2 * Q), 1 - 1e-7);	// 1 at the shadow's edge, over it in float there
	float s2 = (qp + 1) / (qp + 3);
	float K = lensRF(0, 1 - k2, 1);
	float F = sqrt(s2) * lensRF(1 - s2, 1 - k2 * s2, 1);
	return 4 * sqrt(P / Q) * (K - F) - LENS_PI;
}

// psi of a ray at angle theta (ct = cos, st = sin) from the hole; false when the ray falls into it.
bool lensPsi(float ct, float st, float x, out float psi)
{
	psi = acos(clamp(ct, -1, 1));
	if (x <= 0)
		return true;
	float B = st / (x * sqrt(1 - x));	// impact parameter / r_s
	if (ct > 0 && B <= LENS_B_CRIT * (1 + 1e-6))
		return false;	// the edge itself to the shadow: there float gives k2 over 1 (a NaN pixel)
	if (ct > 0.5 && B < 60)
	{
		float sr = min(st / sqrt(1 - x), 1);
		float cr = sqrt(1 - sr * sr);
		psi = asin(sr) - (cr + 1 / cr - 2) / (2 * B) - lensDeflection(B);
	}
	else if (st > 0)
		psi = psi - (1 + ct) / B - (15.0 / 16) * (LENS_PI - psi + st * ct) / (B * B);
	return true;
}

// The direction a pixel's ray comes from (v the pixel's direction, H the hole's, both unit), or false
// when it falls into the hole.
bool lensDir(float3 v, float3 H, float x, out float3 dir)
{
	dir = v;
	float ct = dot(v, H);
	float3 e = v - H * ct;
	float st = length(e);
	float psi;
	if (!lensPsi(ct, st, x, psi))
		return false;
	if (x <= 0 || st < 1e-7)
		return true;	// no hole, or straight at it or away from it
	dir = H * cos(psi) + (e / st) * sin(psi);
	return true;
}

// The angular radius of the shadow (theta of the critical ray).
float lensShadow(float x)
{
	return x > 0 ? asin(min(LENS_B_CRIT * x * sqrt(1 - x), 1)) : 0;
}

#endif
