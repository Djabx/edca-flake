// The blend share of a one-texel memory (edcas_ground_ref.hlsl, edcas_sun_ref.hlsl,
// edcas_canopy_contact.hlsl): each frame the new value goes in by `share` and the old stays by
// `1 - share` (ONE, INV_SRC_ALPHA), so the texel holds a mean over the last `seconds`.
//
// Until client 0.43.146 the shares were constants "at 130 FPS" (0.004, 0.0003, 0.08, 0.0015): at 40
// FPS the fog's level followed the land over 6.5 s instead of 2, the frost grew over 16 s instead
// of 5 and the sun's trend was 80 s old instead of 25 (SHADER-REVIEW.md, 2026-11-11). Now the share
// comes from the frame's length, which edcas.ini keeps in y525 ([Present]: `time` less the last
// frame's `time`, seconds): share = 1 - exp(-dt / seconds), the same memory at any frame rate.

// The frame's length the shares were tuned at, used while y525 is not set (0).
#define MEMORY_DEFAULT_DT (1.0 / 130.0)
// The longest frame that counts in full, seconds: exact from 10 FPS up. A loading hitch (3 s), a pause
// or an alt-tab (a minute) and the first frame (the whole time since the game's start) count as one
// slow frame, so nothing jumps after them. Uncapped, a pause longer than a memory would replace it at
// once by what is in view (the fog's level would jump; the sun's trend, 25 s, would start anew and the
// dawn fog be gone for half a minute); at 1 s a hitch would still move the fog's level by 39 % in one
// frame. Reckoned, not seen in game. The owner's choice of the three, 2026-11-12.
#define MEMORY_MAX_DT 0.1

float memoryShare(float dt, float seconds)
{
	float held = dt > 0 ? min(dt, MEMORY_MAX_DT) : MEMORY_DEFAULT_DT;
	return 1 - exp(-held / seconds);
}
