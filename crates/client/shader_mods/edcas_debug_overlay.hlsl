// A debug overlay for bug reports (edcas.ini [CommandListEdcasOverlay]; y511 = 11 in
// edcas_params.txt shows it, 2026-11-08): bars in the top left corner, drawn into the picture after
// every sky draw and sky upsample of the frame, with last frame's counts of Elite's sky draws and
// of the cloud chain's runs, and the cloud values the shaders see. A screenshot of it tells where
// the chain stopped when the clouds are missing on a machine we cannot capture. The counts are in
// IniParams rows 23 and 24, set from edcas.ini's variables right before the draw.
//
// Bars, top to bottom (counts: 20 px a unit, a tick at each; shares: 200 px for 1):
//  1 small sky draws last frame (orange)     2 full-size sky draws (orange)
//  3 sky upsamples (orange)                  4 the chain: 1 at a sky draw (green), 2 after the
//                                              upsample (yellow), 3 at the late anchor (blue);
//                                              nothing (red, full width)
//  5 bands redrawn after the chain and corrected (orange)
//  6 frames without any sky, of 240 (grey)   7 the last sky draw's cb1[25].w mode, 0 to 3 (white)
//  8 coverage x503 (cyan)   9 opacity x504 (cyan)   10 thickness z505, x505 (cyan)   11 fog x507 (cyan)
Texture1D<float4> IniParams : register(t120);

cbuffer cb1 : register(b1)
{
	float4 cb1[334];
}

void main(float4 pos : SV_Position, out float4 o0 : SV_Target0)
{
	const int bars = 11;
	const float row = 12, width = 260;
	float2 p = pos.xy - float2(8, 8);
	if (p.x < 0 || p.y < 0 || p.x >= width || p.y >= bars * row + 2)
		discard;
	int bar = int(p.y / row);
	float y = p.y - bar * row;
	float4 c23 = IniParams.Load(int2(523, 0)), c24 = IniParams.Load(int2(524, 0));
	float4 p3 = IniParams.Load(int2(503, 0)), p4 = IniParams.Load(int2(504, 0));
	float4 p5 = IniParams.Load(int2(505, 0)), p7 = IniParams.Load(int2(507, 0));
	float3 orange = float3(1, 0.5, 0.1), cyan = float3(0.2, 0.8, 1), grey = float3(0.6, 0.6, 0.6);
	float value = 0, unit = 20;
	float3 colour = orange;
	if (bar == 0) value = c23.x;
	else if (bar == 1) value = c23.y;
	else if (bar == 2) value = c23.z;
	else if (bar == 3) {
		value = c23.w;
		colour = c23.w == 1 ? float3(0.2, 1, 0.2) : c23.w == 2 ? float3(1, 1, 0.2) : float3(0.4, 0.6, 1);
		if (c23.w == 0) { value = width / unit; colour = float3(1, 0.1, 0.1); }
	}
	else if (bar == 4) value = c24.x;
	else if (bar == 5) { value = c24.y / 240; unit = 200; colour = grey; }
	else if (bar == 6) { value = asint(cb1[25].w); colour = 1; }
	else if (bar == 7) { value = p3.x; unit = 200; colour = cyan; }
	else if (bar == 8) { value = p4.x; unit = 200; colour = cyan; }
	else if (bar == 9) { value = p5.z * 5; unit = 200; colour = cyan; }
	else { value = p7.x; unit = 200; colour = cyan; }
	o0 = float4(0.03, 0.03, 0.03, 1);
	if (y >= 1 && y < row - 1 && p.x < value * unit) {
		o0.rgb = colour;
		// a tick at each unit of a count, so the bar can be read as a number
		if (unit == 20 && frac(p.x / unit) < 0.06)
			o0.rgb *= 0.3;
	}
}
