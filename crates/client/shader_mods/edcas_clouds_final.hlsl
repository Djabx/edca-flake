// The cloud shader as the frame's one cloud pass (edcas.ini [CustomShaderEdcasCloudsFinal]), run
// after Elite's last full-size atmosphere draw into a half-size buffer, premultiplied; then
// edcas_clouds_composite.hlsl lays that buffer over the whole picture. The ground fog
// (edcas_fog.hlsl) goes into the same buffer, in front of the clouds or behind them, and the
// wind-blown dust (edcas_dust.hlsl) in front of both. The main gas's high clouds (highClouds, a thin
// shell) go with the clouds, the photochemical haze (photoHaze, closed form) behind clouds and fog.
#define EDCAS_FINAL 1
#define CLOUDS_MAIN cloudsMain
#include "edcas_clouds.hlsl"
#include "edcas_fog.hlsl"
#include "edcas_dust.hlsl"
#include "edcas_lights.hlsl"

void main(float3 v0 : TEXCOORD0, float3 v1 : TEXCOORD1, float3 v2 : TEXCOORD2, float3 v3 : TEXCOORD3,
          out float4 o0 : SV_Target0)
{
	// The pixel's view ray, once for every layer.
	Ray ray = rayOf(v0, v1);
	float4 clouds;
	cloudsMain(v0, v1, v2, v3, ray, clouds);
	// The main gas's high clouds over a trace gas's layer: behind it from below, in front from above.
	bool highInFront;
	float4 high = highClouds(v0, v1, ray, highInFront);
	clouds = highInFront ? high + (1 - high.a) * clouds : clouds + (1 - clouds.a) * high;
	bool inFront;
	float4 fog = groundFog(v0, v1, ray, inFront);
	o0 = inFront ? fog + (1 - fog.a) * clouds : clouds + (1 - clouds.a) * fog;
	// The photochemical haze behind both (closed form, photoHaze).
	float4 haze = photoHaze(v0, v1, ray);
	o0 += (1 - o0.a) * haze;
	// The dust haze (closed form, dustHaze), behind them as well.
	float4 dusty = dustHaze(ray);
	o0 += (1 - o0.a) * dusty;
	float4 dust = groundDust(v0, v1, ray);
	o0 = dust + (1 - dust.a) * o0;
	// Elite's lamps in the cloud and fog, near the camera: in front of both.
	o0.rgb += lightsInMedium(v0, v1, ray);
}
