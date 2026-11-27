// SMAA 1x for the post effects (README.md "Post effects", 2026-11-25): the reference implementation by Jimenez,
// Echevarria, Masia, Navarro and Gutierrez (edcas_smaa_ref.hlsl, https://github.com/iryoku/smaa, MIT), preset
// High (threshold 0.1, 16 search steps, diagonals and corners), colour edges, through its own porting hooks
// (SMAA_CUSTOM_SL): a 3DMigoto custom pass has no sampler of its own (its `sampler =` option makes a comparison
// sampler, not one for plain reads), so every SMAA read is a bilinear or nearest read by hand (edcas_post.hlsl).
// The three passes: edcas_post_smaa_edges.hlsl, edcas_post_smaa_weights.hlsl, edcas_post_smaa_blend.hlsl.
#include "edcas_post.hlsl"

#define SMAA_CUSTOM_SL 1
#define SMAA_PRESET_HIGH 1
// the picture's metrics (1/w, 1/h, w, h), set by each pass's main before it calls SMAA
static float4 smaaMetrics;
#define SMAA_RT_METRICS smaaMetrics
#define SMAATexture2D(tex) Texture2D<float4> tex
#define SMAATexturePass2D(tex) tex
#define SMAASampleLevelZero(tex, coord) BilinearAt(tex, coord)
#define SMAASampleLevelZeroPoint(tex, coord) PointAt(tex, coord)
#define SMAASampleLevelZeroOffset(tex, coord, offset) BilinearAt(tex, (coord) + float2(offset) * SMAA_RT_METRICS.xy)
#define SMAASample(tex, coord) BilinearAt(tex, coord)
#define SMAASamplePoint(tex, coord) PointAt(tex, coord)
#define SMAASampleOffset(tex, coord, offset) BilinearAt(tex, (coord) + float2(offset) * SMAA_RT_METRICS.xy)
#define SMAA_FLATTEN [flatten]
#define SMAA_BRANCH [branch]
#include "edcas_smaa_ref.hlsl"

// The metrics of the picture and the pixel's texture coordinate.
float2 SmaaSetUp(float4 pos)
{
	float4 size = PictureSize();
	smaaMetrics = float4(size.zw, size.xy);
	return pos.xy * size.zw;
}
