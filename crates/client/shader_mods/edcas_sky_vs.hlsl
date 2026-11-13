// One triangle over the whole picture carrying what the sky pass's vertex shader (41b3ec0bbef1e22c)
// gives the cloud chain's pixel shaders, for a frame in which Elite makes no full-size sky draw to
// hang the chain on (edcas.ini [CommandListEdcasChainFull], 2026-11-08): the chain then runs after
// Elite's upsample of its small sky buffer, whose vertex shader is another one.
//
// The sky pass draws the atmosphere's shell, a unit sphere in its model space, and its vertex
// shader puts out per vertex: TEXCOORD0 the clip position's x, y and w; TEXCOORD1 the vertex
// relative to the camera in world metres (the view ray); TEXCOORD2 the vertex times the atmosphere
// radius; TEXCOORD3 the vertex itself. Its cb0 holds the model-to-clip rows 4..7 and the
// model-to-camera-relative-world rows 9..11. edcas.ini copies that buffer at every sky draw
// (ResourceEdcasSkyVsCb0) and binds it here as cb0.
//
// A pixel's ray in the model space is the line of the points whose clip x and y are the pixel's:
// the planes (row4 - ndc.x row7) . X = 0 and (row5 - ndc.y row7) . X = 0, which meet in a line
// through the eye. Its direction is the cross product of their normals, linear in ndc (the
// ndc.x ndc.y term is row7 x row7 = 0), so the triangle's three corners interpolate it exactly over
// the picture; rows 9..11 turn it into the world. Only its direction is used: every reader
// normalises v1 or divides it by its own depth (viewRay in edcas_clouds.hlsl). TEXCOORD0 is
// (-ndc, -1): the readers' v0.xy / v0.z is the ndc, as with the pass's own, and the negative v0.z
// tells viewRay to work the pass's surface depth out from the shell.
cbuffer cb0 : register(b0)
{
	float4 cb0[12];
}

void main(uint vertex : SV_VertexID, out float4 position : SV_Position, out float3 v0 : TEXCOORD0,
          out float3 v1 : TEXCOORD1, out float3 v2 : TEXCOORD2, out float3 v3 : TEXCOORD3)
{
	float2 uv = float2((vertex << 1) & 2, vertex & 2);
	float2 ndc = uv * float2(2, -2) + float2(-1, 1);
	position = float4(ndc, 0.5, 1);
	v0 = float3(-ndc, -1);
	float3 n1 = cb0[4].xyz - ndc.x * cb0[7].xyz;
	float3 n2 = cb0[5].xyz - ndc.y * cb0[7].xyz;
	float3 d = cross(n1, n2);
	// Forwards: the clip w grows along the ray (the eye is at w = 0).
	if (dot(d, cb0[7].xyz) < 0)
		d = -d;
	v1 = float3(dot(cb0[9].xyz, d), dot(cb0[10].xyz, d), dot(cb0[11].xyz, d));
	v2 = 0;
	v3 = 0;
}
