// Elite's local lamps (ship's and SRV's lights, suit torch, hull, cockpit and settlement lamps), as
// edcas.ini copies them after Elite's light-volume draws: their records (t114..t116), the draws'
// view constants (b13) and how many lamps each draw had (IniParams x510, y510, z510). For the lamps'
// light in cloud and fog (edcas_lights.hlsl) and on the canopy frost (edcas_canopy_ice.hlsl).
// Record format and Elite's falloff: README-ELITE.md, "Lights". The includer declares IniParams.

// Records of 112 bytes (spot lights with a shadow matrix) and 48 (small lamps).
struct Record112 {
	uint4 q[7];
};
struct Record48 {
	uint4 q[3];
};
StructuredBuffer<Record112> Spots : register(t114);   // PS a5d9ece9c5ceccf7: the ship's lights
StructuredBuffer<Record112> Spots2 : register(t115);  // PS 0d055cd72839102e: seen on foot
StructuredBuffer<Record48> Small : register(t116);    // PS 4034d618cc7fce8a: hull and cockpit lamps
// The light draws' view constants (their vs-cb2): [2..4] world -> view, [46].z colour scale.
cbuffer LightView : register(b13)
{
	float4 lightView[47];
}

struct Light {
	float4 at;      // position relative to the camera (world axes, metres), range
	float4 aim;     // direction it shines in (world axes), cosine of the cone's outer edge
	float4 colour;  // colour x intensity (Elite's lighting units), cone exponent
	float4 cut;     // near cut-off squared, source radius squared, cosine of the inner edge
};

// One of Elite's light records (the first 48 bytes, the same in both sizes):
//   +0  float3 position (world, metres), float range
//   +16 uint lo16 / hi16: source radius and near cut-off, as shares of the range (x / 65535);
//       float intensity; uint colour, 10 bits each (red low), x intensity / 1023; uint direction:
//       bits 0-1 axis order, bit 2 sign of the major axis, bits 3-16 and 17-31 the two others
//   +32 half floats: hi of +32 cos(outer edge), hi of +36 cos(inner edge), lo of +44 exponent
Light decode(uint4 q0, uint4 q1, uint4 q2, float3 camera)
{
	Light l;
	float range = asfloat(q0.w);
	l.at = float4(asfloat(q0.xyz) - camera, range);
	uint d = q1.w;
	float3 v = normalize(float3((d & 4) ? -1 : 1,
	                            max(float((int)d >> 17) * 0.0000610388815, -1),
	                            max(float((int)(d << 15) >> 18) * 0.000122085214, -1)));
	uint order = d & 3;
	v = order == 0 ? v : (order == 1 ? v.zxy : v.yzx);
	l.aim = float4(v, f16tof32(q2.x >> 16));
	float3 rgb = float3(q1.z & 1023, (q1.z >> 10) & 1023, (q1.z >> 20) & 1023);
	l.colour = float4(rgb * asfloat(q1.y) * lightView[46].z / 1023, f16tof32(q2.w & 0xffff));
	float source = (q1.x & 0xffff) * range / 65535;
	float nearCut = (q1.x >> 16) * range / 65535;
	l.cut = float4(nearCut * nearCut, source * source, f16tof32(q2.y >> 16), 0);
	return l;
}

// How many lamps each copied draw had this frame (0 without the draw).
void lampCounts(out uint spots, out uint spots2, out uint small)
{
	float4 counts = IniParams.Load(int2(510, 0));
	spots = (uint)counts.x;
	spots2 = (uint)counts.y;
	small = (uint)counts.z;
}

// The camera in world space, from the light draws' world-to-view matrix: -(R^T t).
float3 lampCamera()
{
	return -(lightView[2].xyz * lightView[2].w + lightView[3].xyz * lightView[3].w + lightView[4].xyz * lightView[4].w);
}

// Lamp i of all three draws, in that order.
Light lamp(uint i, uint spots, uint spots2, float3 camera)
{
	if (i < spots)
		return decode(Spots[i].q[0], Spots[i].q[1], Spots[i].q[2], camera);
	if (i < spots + spots2)
		return decode(Spots2[i - spots].q[0], Spots2[i - spots].q[1], Spots2[i - spots].q[2], camera);
	uint j = i - spots - spots2;
	return decode(Small[j].q[0], Small[j].q[1], Small[j].q[2], camera);
}

// Lamp i's place relative to the camera and its range (`decode`'s `at`), from the record's first 16
// bytes alone: enough to test whether a ray passes through its reach before decoding the rest.
float4 lampAt(uint i, uint spots, uint spots2, float3 camera)
{
	uint4 q0;
	if (i < spots)
		q0 = Spots[i].q[0];
	else if (i < spots + spots2)
		q0 = Spots2[i - spots].q[0];
	else
		q0 = Small[i - spots - spots2].q[0];
	return float4(asfloat(q0.xyz) - camera, asfloat(q0.w));
}

// The light a lamp sheds on a point (camera-relative world, metres), facing it, as Elite lights a
// surface: colour x cone x window / max(d^2, source radius^2); nothing within the near cut-off.
float3 lampLightAt(Light l, float3 p, out float3 toLamp)
{
	float3 fromLamp = p - l.at.xyz;
	float d2 = dot(fromLamp, fromLamp);
	toLamp = -fromLamp * rsqrt(max(d2, 1e-6));
	float r = l.at.w;
	if (!(r > 0) || d2 < l.cut.x || d2 > r * r)
		return 0;
	float cone = pow(saturate((dot(-toLamp, l.aim.xyz) - l.aim.w) / max(l.cut.z - l.aim.w, 1e-4)), l.colour.w);
	float near2 = max(d2, l.cut.y);
	float window = max(1 - pow(near2 / (r * r), 2), 0);
	return l.colour.rgb * cone * window * window / near2;
}
