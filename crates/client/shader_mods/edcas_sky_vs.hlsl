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
// normalises v1 or divides it by its own depth (viewRay in edcas_clouds.hlsl), so it is scaled to a number
// near 1 (below). TEXCOORD0 is
// (-ndc, -1): the readers' v0.xy / v0.z is the ndc, as with the pass's own, and the negative v0.z
// tells viewRay to work the pass's surface depth out from the shell.
cbuffer cb0 : register(b0)
{
	float4 cb0[12];
}

// The position last: DXVK links the stages by register, not by name, and the readers take TEXCOORD0
// to 3 in registers 0 to 3, where Elite's sky vertex shader writes them. With the position first they
// went to 1 to 4: the pixel shaders read v0 = 0 and the ray (-ndc, -1) as v1, a view fixed to the
// screen, and the dust's billows hung round the camera wherever it looked (ed_frame133319,
// 2026-11-23; since this path was made on 2026-11-08, seen once client 0.43.259 ran it every frame).
void main(uint vertex : SV_VertexID, out float3 v0 : TEXCOORD0, out float3 v1 : TEXCOORD1,
          out float3 v2 : TEXCOORD2, out float3 v3 : TEXCOORD3, out float4 position : SV_Position)
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
	// One scale for the whole draw (the same for the three corners, so the interpolation stays exact): the rows
	// are in metres times millions, and the ray came to some 1e19 from orbit, whose square (length,
	// normalize, 1e39) is past a float's 3.4e38. Its length was infinite, normalize(v1) 0: the pixel's angle
	// (fwidth over length) 0, so the clouds' noise and waves went unfiltered and the high clouds glittered over
	// the whole planet, and the shell's own depth (viewRay) 0, so the planet's ground was never seen
	// (ed_frame222039, 2026-11-26). Scaled by the rows' sizes it is a number near 1.
	float scale = 1 / max(length(cb0[4].xyz) * length(cb0[5].xyz) * length(cb0[9].xyz), 1e-30);
	v1 = float3(dot(cb0[9].xyz, d), dot(cb0[10].xyz, d), dot(cb0[11].xyz, d)) * scale;
	v2 = 0;
	v3 = 0;
}
