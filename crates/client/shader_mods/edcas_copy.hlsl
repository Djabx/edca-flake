// Copies a picture onto the render target pixel for pixel (edcas.ini, 2026-11-08): the picture
// under the clouds put back where Elite is about to redraw a band of its sky after the cloud chain
// has run ([CustomShaderEdcasRestore], the draw's own stencil limiting it to the band, SV_Target1
// marking the band in ResourceEdcasBandMask), and the clean copy brought up to date with the
// band's new sky after that draw ([CustomShaderEdcasCleanUpdate], within the mark: x525 = 1, since
// the stencil may not say the same after the game's draw).
Texture2D<float4> Picture : register(t110);
Texture2D<float> Mask : register(t111);
Texture1D<float4> IniParams : register(t120);

void main(float4 pos : SV_Position, out float4 o0 : SV_Target0, out float o1 : SV_Target1)
{
	int3 pixel = int3(pos.xy, 0);
	if (IniParams.Load(int2(525, 0)).x > 0 && Mask.Load(pixel) <= 0)
		discard;
	o0 = Picture.Load(pixel);
	o1 = 1;
}
