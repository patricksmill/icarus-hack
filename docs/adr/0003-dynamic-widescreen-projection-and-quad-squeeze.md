# 3. Dynamic Widescreen Projection and Quad Squeeze

The 3DS hardware top screen has a fixed 5:3 (400x240) aspect ratio. On PC, displays vary from 16:9 to 21:9 or arbitrary window dimensions. A naive viewport resize either stretches 3D geometry or adds letterboxing.

We decided to:
1. Dynamically update the camera Lens Aspect at `camera+0x40` (with dirty flag bit 2 at `camera+8` cleared) whenever window dimensions change, keeping 3D frustum culling (`0x00756754`) and aim rays (`0x00360168`) synchronized with the visible geometry.
2. Maintain 2D HUD proportions across arbitrary window widths via Pass-Level Quad Squeeze in `0x0092F408`: scaling centered UI quads horizontally by $1/k$ (where $k = \text{aspect} / 1.6667$), while placing edge-anchored status elements directly at viewport boundaries.

This achieves genuine widescreen rendering without stretching UI elements or causing frustum culling pop-in at screen edges.
