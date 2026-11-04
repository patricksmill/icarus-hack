# 1. Native UI Pass Replay

The 3DS original puts HUD, dialogue illustrations, and menus on the bottom screen (320x240). On PC, gameplay occurs on a single high-resolution display without a mandatory second window.

We decided to replay bottom-screen UI draw calls directly inside the native top screen passes (0x0092E4F8) with edge anchoring, rather than capturing and compositing the bottom framebuffer.

Native replay keeps text, portraits, and icons crisp at any rendering scale and widescreen aspect ratio, whereas framebuffer blitting produces pixelated 240p artifacts and breaks resolution independence.
