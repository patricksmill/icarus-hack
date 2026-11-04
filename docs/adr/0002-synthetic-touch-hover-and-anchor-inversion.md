# 2. Synthetic Touch Hover and Anchor Inversion

3DS touch menus expect stylus events (`touching`, `began`, `x`, `y`) mapped across a fixed 320x240 touch digitized coordinate field. On PC, mouse navigation requires hover highlighting, resolution independence, and widescreen edge-anchored layouts.

We decided to:
1. Synthesize touch state from mouse movement as a held touch (`touching=1, began=0`), firing activation (`began=1`) exclusively on mouse button down events. When the mouse leaves interactive UI zones, out-of-bounds coordinates (`-1, -1`) are sent to clear highlights.
2. Translate PC mouse coordinates into hit-test boxes via Anchor Space Inversion: subtracting element displacement anchors before checking against the game's native bounding boxes.

This allows all menus (in-battle Pause/Options/Dialogs and out-of-battle Scripted Menus) to feel native to desktop mouse users without rewriting the underlying game state machines or hardcoding display resolutions.
