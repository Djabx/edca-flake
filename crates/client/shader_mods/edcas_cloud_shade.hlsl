// The clouds' shadow on the ground (edcas.ini [CustomShaderEdcasCloudShade], client 0.43.224): how much of
// the direct sunlight the cloud slab takes from what each pixel shows, 0 (none) to 1, into a picture-size
// buffer. It runs right before Elite's sun pass, on that pass's own full-screen draw, and our copy of the
// sun pass takes it off the sun's colour (star_light.py cloud_block), as it does the sunset on the ground:
// so the shadow darkens the sunlight alone, with Elite's own shadows, and not the sky's light, the lamps'
// or what glows.
//
// Seen from above, a thin cloud over sunlit ground shows by its shadow more than by its own light: the
// cloud of `ed_frame36863.rdc` (optical depth 0.25, the sun 12 degrees up) lifts the ground under the
// camera by a fifth, and its shadow takes half the sunlight off the ground it falls on (README.md, "A thin
// cloud seen from above is faint").
//
// What it has to go on. The sun pass has no depth and no planet, so:
//   v0, v1    the sun pass's own vertex outputs: the pixel's place on the screen (0 to 1) and its view ray
//             in the world's axes, 1 long along the view's forward axis cb2[4] (the four corners of
//             `ed_frame36863.rdc`: dot(v1, cb2[4].xyz) = 1.000)
//   cb2       the sun pass's constants, as bound: [1].xy the picture's size, [4] the forward axis,
//             [42] the direction the sunlight travels, in the world's axes (the sky pass's cb1[4] turned
//             back by cb1[19..21]; [2..4] = cb1[277..279])
//   t115      the frame's linear depth, metres along the forward axis: the texture Elite's shadow pass
//             77cb0c225019d08b reads as t0 some draws before the sun pass, and the sky pass reads later
//   b13       the sky pass's constants of the frame before (ResourceEdcasSkyCb1): the camera's place over
//             the planet [3], the turn into the planet's frame [19..21] and the planet's size [31]. The sky is drawn after the lighting, so this
//             frame's are not there yet; a frame's flight moves a shadow by metres, where a cloud is
//             kilometres wide. The view's turn is this frame's (v1), so the shadows do not lag a turning
//             head.
// The sky pass's planet frame is the world's turned by cb1[19..21] (a rotation times the atmosphere radii
// per metre, .w the camera = cb1[3]): a point is those rows times (v1 * depth, 1), and the sun's direction
// (cb2[42], the world's axes) is turned by them too. Near the planet the rotation is the identity, so until
// 2026-11-27 this took the world's axes as the planet's: from 294 km up (`ed_frame89840.rdc`)
// the rows are a real rotation, the ground points came out hundreds of km off and the shadow fell only in
// a wedge with a hard straight edge across the planet, gone at 210 km (`ed_frame95339.rdc`: the identity).
//
// Not shaded: what is above the slab's top (slabShadow), the thin shell (z505 = 0), the second, high layer
// (x521: optical depth 0.04 to 0.10). z502 is the strength (0 = off: the pass does not run).
#define CLOUDS_MAIN cloudsMain
#define EDCAS_CB1_SLOT b13
#define EDCAS_DEPTH_SLOT t115
#include "edcas_clouds.hlsl"

cbuffer cb2 : register(b2)
{
	float4 cb2[47];
}

void main(float2 v0 : TEXCOORD0, float3 v1 : TEXCOORD1, out float4 o0 : SV_Target0)
{
	o0 = 0;
	float strength = saturate(IniParams.Load(int2(502, 0)).z);
	// No planet in the kept constants (the first frame at one): nothing to place a cloud by.
	if (strength <= 0 || !(cb1[31].w > 0) || !(cb1[31].z > 0))
		return;
	// The pixel as the sun pass finds its own in the G-buffer.
	int2 pixel = int2(min(floor(v0 * cb2[1].xy), cb2[1].xy - 1));
	float depth = Depth.Load(int3(pixel, 0)).x;
	float along = dot(v1, cb2[4].xyz);
	if (!(depth > 0) || !(along > 0))
		return;
	// Into the planet's frame as the sky pass turns its own view ray: rows 19 to 21 (a rotation times the
	// atmosphere radii per metre, the camera in .w). Far out they are not the identity (2026-11-27).
	float4 view = float4(v1 / along * depth, 1);
	float3 p = float3(dot(cb1[19], view), dot(cb1[20], view), dot(cb1[21], view));
	// Past the air's top is the sky, or space: no ground there to shade.
	if (dot(p, p) >= 1)
		return;
	// The sunlight's direction (the world's axes) turned the same way; normalize takes the scale off.
	float3 s = -cb2[42].xyz;
	float3 sun = normalize(float3(dot(cb1[19].xyz, s), dot(cb1[20].xyz, s), dot(cb1[21].xyz, s)));
	o0 = slabShadow(p, sun) * strength;
}
