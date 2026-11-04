# High Refresh Mode by presenter interpolation, game locked at 60 updates

High Refresh Mode keeps the game updating 60 times a second (the 60 FPS patch) and has zakuro's presenter draw pictures in between the last two real frames at the host's refresh rate. We did not run the game's own update faster with a smaller timestep, even though its actors already take a frame-scaled dt: a native 120 Hz spike (analysis/PERFORMANCE.md §5) kept movement and HUD timers correct but desynced dialogue audio, because the game's sound code steps once per update, and dropped frames from the doubled per-refresh work. Holding the update at a fixed 60 also keeps the game deterministic per update, which a future lockstep multiplayer mode would need.

## Consequences

- Up to one 60th of a second of extra display latency, the cost of interpolating rather than predicting.
- Gameplay, audio and timing are unchanged by construction; only the picture changes.
- It replaces zakuro's 30-to-60 "Frame interpolation" setting: a 30 FPS game at 60 Hz is the same drawer at the halfway point, so there is one mode and one code path.
