// Ice on the canopy while the ship flies through cloud or fog (edcas.ini runs it right after
// Elite's cockpit glass, pixel shader 953f692f27531387, on its geometry).
//
// Frost: the glass is cold (thin air takes little heat away, and it radiates into a cold sky), so
// the cloud's crystals freeze onto it, first along the frame, and the frost creeps inwards the
// longer the ship stays in cloud (edcas_canopy_frost.hlsl, over about 5 s); out of the cloud it
// sublimates the same way. Where it grows follows Elite's own glass mask (the vertex shader's
// COCKPITGLASSMASKS y and z: high along the frame and the struts, frayed, black mid-glass); its
// crystals are ridged noise on the glass's texture coordinates, so they stay on the glass.
// While the ship is in cloud right now (edcas_canopy_contact.hlsl) a faint mist lies on the glass.
// White specks for the crystals that sublimate on impact read as random dots, not ice, and slid
// with head look on their screen grid (2026-10-27): left out.
//
// IniParams row 9: x509 strength (0 = 1), y509 brightness (0 = 4).

Texture2D<float4> Contact : register(t112);
Texture2D<float4> Frost : register(t113);
Texture1D<float4> IniParams : register(t120);
// Still bound from Elite's glass draw: its light list of directional lights, the sun first (colour
// at byte 0, the direction its light travels at byte 16, in view space), README-ELITE.md.
ByteAddressBuffer GlassLights : register(t1);

cbuffer cb1 : register(b1)
{
	float4 cb1[327];
}

#include "edcas_lamps.hlsl"

// How much brighter frost makes the light already at the glass, per unit of its scattering: it
// passes half the direct view on and scatters the light around it back, about twice what one
// direction brings (the cockpit's light comes from all round).
#define FROST_VEIL 1.5

// The cockpit's light: the picture before the glass, shrunk to a sixteenth and blurred
// (edcas_cockpit_light.hlsl).
Texture2D<float4> CockpitLight : register(t117);

// The small picture at a point, bilinear (no sampler of ours is bound).
float3 cockpitAt(float2 texel, float2 size)
{
	float2 at = clamp(texel - 0.5, 0, size - 1);
	int2 base = int2(floor(at));
	float2 f = at - base;
	int2 far = min(base + 1, int2(size - 1));
	float3 a = CockpitLight.Load(int3(base, 0)).rgb, b = CockpitLight.Load(int3(far.x, base.y, 0)).rgb;
	float3 c = CockpitLight.Load(int3(base.x, far.y, 0)).rgb, d = CockpitLight.Load(int3(far, 0)).rgb;
	return lerp(lerp(a, b, f.x), lerp(c, d, f.x), f.y);
}

// The light of the cockpit around a point of the glass: the mean of eight points of the shrunk
// picture on a ring about 3 % of the picture's width out. Lit frame, panels and screens nearby
// light the frost; with only the night outside around, it stays dark. The emissive panels and
// Elite's ambient that light the cockpit are in no light list (2026-10-28). On the scene's scale
// already, so it follows Elite's exposure like everything else.
float3 cockpitLight(float2 pixel)
{
	uint width, height;
	CockpitLight.GetDimensions(width, height);
	float2 size = float2(width, height);
	float2 texel = pixel / 16;
	float radius = 0.03 * width;
	float3 sum = 0;
	[unroll] for (int k = 0; k < 8; k++) {
		float angle = k * 0.7853982 + 0.39;
		sum += cockpitAt(texel + float2(cos(angle), sin(angle)) * radius, size);
	}
	return sum / 8;
}

#include "edcas_noise.hlsl"

// The light on the frost at a point of the glass, on the scene's scale (x cb1[90].y, as Elite's
// glass): the sun as the glass gets it (about 1e-13 at night) and Elite's lamps (cockpit, hull
// and ship's lights, edcas_lamps.hlsl), from either side as frost scatters through itself, over
// pi as a white diffuse surface. No light of its own: a fixed minimum glowed white at night under
// Elite's exposure, like the fog's ambient did (2026-10-28).
float3 frostLight(float3 normal, float3 position)
{
	float3 n = normalize(normal);
	float3x3 toView = float3x3(cb1[277].xyz, cb1[278].xyz, cb1[279].xyz);
	float3 sunColour = asfloat(GlassLights.Load3(0)) * starLightTint();
	float3 sunTravel = normalize(asfloat(GlassLights.Load3(16)));
	float3 e = sunColour * (0.35 + 0.65 * abs(dot(normalize(mul(toView, n)), sunTravel)));
	uint spots, spots2, small;
	lampCounts(spots, spots2, small);
	float3 camera = lampCamera();
	[loop] for (uint i = 0; i < spots + spots2 + small && i < 32; i++) {
		float3 toLamp;
		float3 lit = lampLightAt(lamp(i, spots, spots2, camera), position, toLamp);
		e += lit * (0.35 + 0.65 * abs(dot(n, toLamp)));
	}
	return e * (cb1[90].y / 3.14159265);
}

// The inputs are the cockpit glass vertex shader's outputs (4a0eeed36eec4aff), in its order.
void main(float3 masks : __USER_KEY_COCKPITGLASSMASKS0,
          nointerpolation uint face : __USER_VERTEX_FACEINVARIANT0,
          float3 normal : __USER_VERTEX_M_LIGHTINGNORMAL0,
          float3 position : __USER_VERTEX_M_LIGHTINGPOSITION0,
          float3 tangent : __USER_VERTEX_M_LIGHTINGTANGENT0,
          float2 uv : __USER_VERTEX_M_TEXCOORD0,
          float4 screen : SV_Position,
          out float4 o0 : SV_Target0,
          out float4 glassLight : SV_Target1)
{
	float4 params = IniParams.Load(int2(509, 0));
	float strength = params.x > 0 ? params.x : 1;
	float4 c = Contact.Load(int3(0, 0, 0));
	float contact = (c.y > 0 ? c.x / c.y : 0) * strength;
	float4 f = Frost.Load(int3(0, 0, 0));
	float frost = saturate((f.y > 0 ? f.x / f.y : 0) * strength);
	if (contact < 0.01 && frost < 0.01)
		discard;
	// White frost scatters about 85 % of the light it gets. Drawn with two outputs (edcas.ini:
	// blend ONE, SRC1_COLOR, dual source): o0 adds what it scatters of the sun and Elite's lamps,
	// and glassLight scales what the glass already shows, the view through it and the cockpit's
	// light on it (emissive panels, Elite's ambient, which reach the glass as its reflections and
	// are in no light list): the frost takes away part of the direct view and scatters the light
	// at the glass back as a veil. So unlit frost stays see-through, and frost the cockpit's light
	// reaches shows; with the sun and lamps alone it was nearly invisible at night (2026-10-28).
	// The cockpit's surfaces cover about half of what the frost sees from inside.
	// y509 scales how bright the frost is (0 = 4): at 1 it was nearly invisible at night beside the lit
	// frame (the 16 x 16 blur mixes the narrow frame with the dark around it); 1, 2, 4 and 8 tried
	// live, 4 looked right (2026-10-28).
	float shine = params.y > 0 ? params.y : 4;
	float3 lit = 0.85 * shine * (frostLight(normal, position) + 0.5 * cockpitLight(screen.xy));

	// The frost: everything along the frame whose mask is above the front, which moves inwards
	// as the frost grows; the crystals fray the front and give the frost its grain.
	float edge = max(masks.y, masks.z);
	// Where the mask is too low for the front even with the crystals' push, only the mist: the
	// crystals cost 16 noise lookups a pixel over a canopy that fills the screen (140 -> 100 FPS
	// in cloud, 2026-10-27, part of it the march inside the cloud), and the middle of the glass
	// never frosts.
	float front = 1 - 0.9 * pow(frost, 0.6);
	if (edge < front - 0.1) {
		float mist = contact * 0.04;
		o0 = float4(lit * mist, 0);
		glassLight = 1 + FROST_VEIL * mist;
		if (mist <= 0)
			discard;
		return;
	}
	// Frost grows in needles across the frame, away from it. The glass's texture coordinates run
	// in rings round the nose and the frame lies along those rings, so across the frame is across
	// the rings: y (with x the streaks lay along the frame, 2026-10-27). (Plain noise on the coordinates curled along the frame like smoke;
	// the mask's own slope as the direction is constant on each triangle of the mesh and drew its
	// facets, 2026-10-27.)
	float2 across = float2(0, 1);
	// The crystals: ridged noise smeared along that direction (six samples in a row, over about
	// two crystals; over one they formed a net of veins, 2026-10-27), so each ridge becomes a
	// needle pointing across the frame. At 5 per unit the grain was coarse and the
	// thick frost foamed; 10.
	// At 10 per unit the needles looked coarse, like cracks (2026-10-28): 16, with eight samples
	// half a unit apart, so they stay as long on the glass.
	float2 g = uv * 16;
	float crystal = 0, amp = 0.5;
	// Two layers: the third, at an eighth of the strength, would cost another 8 noise lookups a pixel.
	[unroll] for (int o = 0; o < 2; o++) {
		float n = 0;
		[unroll] for (int k = 0; k < 8; k++)
			n += noise(float3(g + across * (0.48 * k), o * 7.3));
		n = saturate(0.5 + (n / 8 - 0.5) * 1.8);  // the mean of eight flattens the noise: its contrast back
		crystal += amp * pow(1 - abs(2 * n - 1), 4);  // ridges: thin bright lines (power 3 drew them broad)
		g = g * 2.3 + float2(3.1, 7.7);
		amp *= 0.5;
	}
	crystal *= 0.875 / 0.75;  // to the three layers' range, which the front and alpha were tuned on
	// The front follows the frost on a curve (x509 = 0.6 left a few crystals where 1 had covered
	// the frame, 2026-10-27). The crystals fray it only where the mask is: on their own they
	// crossed the front at full frost and scattered curved strokes over the middle of the glass.
	// A crisp front, the crystals pushing it a little either way: with a wide ramp and a strong
	// push the frost faded out like haze and single long curls stood off the frame (2026-10-28).
	float cover = smoothstep(front - 0.02, front + 0.08, edge + (crystal - 0.5) * 0.12 * saturate(edge * 8));
	// See-through even where thick, grain from the crystals: at up to 0.85 alpha, white, the
	// thick frost along the bottom of the frame looked like breaking waves (2026-10-27).
	float scatter = max(contact * 0.04, cover * (0.25 + 0.4 * crystal) * 0.75);
	o0 = float4(lit * scatter * float3(0.95, 0.98, 1), 0);
	glassLight = 1 + FROST_VEIL * scatter * float4(0.95, 0.98, 1, 0);
	if (scatter <= 0)
		discard;
}
