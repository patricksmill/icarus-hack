# Performance review — 30 October 2026

Target: i5-13420H / RTX 3050 Laptop, 1920×1080, over 100 **distinct game
frames per second at normal game speed**.

There are useful optimization targets, but this review does not establish
that 1080p/100+ is achievable. The initial measurements below preceded the
GPU-local color surface alias optimization documented at the end. That
optimization changes the renderer, not the installed game DLL or game timing.

## What is established

- The Windows `zakuro.log` selects the RTX 3050, Vulkan, shared-device
  presentation, recompiled game code, 3× internal resolution, and 2× emulated
  CPU overclock. This is not an accidental software-rendering configuration.
- 3× means a 720-pixel-tall internal image, subsequently scaled to the window.
  The renderer's scale is an integer. 4× is 960 tall; 5× is 1200 tall. Going
  from 3× to 5× multiplies render-target pixel counts by 25/9, before any
  optimizations. A 1080p window by itself does not imply 1080p rendering.
- The frontend schedules normal play at approximately 59.83 console refreshes
  per second: `CYCLES_PER_FRAME = 4_481_136`, and `FRAME_TIME` derives from
  the emulated CPU clock. Fast forward changes game speed. The FPS counter
  counts emulated refreshes, not distinct pictures produced by the game.
- The existing 60 FPS patch changes the game's present interval, leftover
  vblank handling, update count, and timestep. See `make_60fps_ips.py` and
  FINDINGS.md. It is not a general variable-framerate patch.

## Measurements and their limits

### Repeatable user-run baseline, 14:46 EDT

The user ran the Windows benchmark at 3× on the RTX 3050 and supplied the
complete log, `kiu-perf-fed726db8ccc4e8b82ef8db0fcd72216/results.log`:

| Run | Refreshes | Wall time | Mean time | Throughput |
| --- | --- | --- | --- | --- |
| 1 | 300 | 5.234 s | 17.45 ms | 57.3 refreshes/s |
| 2 | 300 | 5.183 s | 17.28 ms | 57.9 refreshes/s |

The combined mean is 17.36 ms/refresh, with approximately 1% difference
between runs. Both execute identical instruction totals: 1,312,992
interpreted and 278,795,895 recompiled. This is the preferred baseline for
subsequent comparisons. It excludes window presentation and audio output.

GPU reports emitted during the timed blocks show 654–673 ms per 600
batches, of which 374–410 ms is labelled download, and 619–636 ms of host
completion waits across 511–515 waits. The batches do not align exactly
with timed blocks. These figures do not establish CPU or GPU wall-time
shares, or prove that removing waits would save the same amount of time.

Reaching 100 refreshes/s requires reducing the combined mean by about 42%,
before accounting for higher internal resolution and high-refresh timing
changes.

### Resolution control, 14:49 EDT

At 1× the user measured 4.728 and 4.622 seconds for the same two
300-refresh blocks: **15.58 ms/refresh, 64.2 refreshes/s combined**. Both
instruction totals match the 3× baseline exactly. Reducing pixel counts by
9× reduced frame time by only **10.24%**, or 1.78 ms.

GPU fragment/compute invocation counts fell by approximately 9×, while
completion waits remained around 509–515 per 600 batches. Most cost in
this scene is insensitive to resolution. This does not distinguish CPU
computation from submission, conversion and synchronization costs by itself.

### Host path timing and readback attribution, 14:59 EDT

Added opt-in host wall timers and readback attribution, then ran the same
Windows scene at 3×. The log is
`kiu-perf-d9f268313fe54bcf98350e6f04b90cea/results.log`.

| Measured path | Run 1, ms/refresh | Run 2, ms/refresh |
| --- | --- | --- |
| Entire instrumented run | 19.73 | 20.59 |
| Compiled game code | 2.728 | 2.691 |
| Interpreter | 0.412 | 0.296 |
| Host hooks | 0.179 | 0.207 |
| Audio/devices | 0.062 | 0.069 |
| Service IPC, SVC 0x32 | 15.626 | 16.601 |
| GPU command lists, nested inside IPC | 14.954 | 15.976 |

These are wall times including waits inside each path, not CPU utilization
or a sampled call-stack profile. Instrumentation adds overhead; the slower
total is not a game optimization result. Both instruction totals still
match the uninstrumented baseline. GPU command processing is the dominant
measured path; hook callbacks are a much smaller target in this scene.

Every recorded nonempty readback was for overlapping surfaces or texture
consumption. Across representative 600-batch windows:

- Overlapping surfaces: 256–258 calls, approximately 123 MiB of new
  downloads, and 679–902 ms of host wall time. Source buffers include
  `0x1F000000`, `0x1F300000`, `0x1F304000`, and `0x1F305000`.
- Texture reads: 85–86 calls, approximately 43 MiB downloaded from
  `0x1F080000`, and 166–202 ms of host wall time.
- CPU conversion and guest stores: approximately 250–346 ms, **nested
  inside** the readback times above.

These windows include batch-boundary effects and do not align exactly with
the timed blocks. They establish an optimization target, not the time a fix
will save. The next implementation target is GPU-local resolution of
overlapping surface representations and render-to-texture transitions,
preserving layout, format conversion and untouched regions. Required GPU
data should remain on the GPU until a genuine CPU consumer needs it.

### Earlier exploratory measurements

Built and ran a Windows kiutrace executable against the **installed Windows
game DLL and native NVIDIA Vulkan driver**, using `trace/states/1.state`,
the saved 60 FPS patch, 2× CPU overclock, 16:9, HUD 0.6, narration, portraits,
mirrored UI and pause UI. The library was checked against the state's code
before running: three functions differ because of the 60 FPS patch.

The first 300-refresh timed block, after 90 warm-up refreshes, took
6.428 seconds: **21.43 ms/refresh, 46.7 refreshes/s**. This reproduces the
reported range, without the window or display wait.

In that run, 600-batch GPU reports included approximately:

| Measurement | Observed range |
| --- | --- |
| GPU time | 569–588 ms |
| Time labelled download/readback by the GPU instrumentation | 250–254 ms |
| Host wait for completed GPU work | 522–547 ms |
| Completed-work waits | 507–515 |
| Barriers | 7,903–8,160 |
| Render passes | 2,833–3,083 |
| Draws | 13,885–14,434 |

These are totals **per 600 GPU batches**, not per 600 game frames. GPU work
and CPU work can overlap; the timings must not be added together. Timing
categories can include work between their instrumentation markers.

Later repetitions of the same scene took about 51 ms/refresh, so the machine
was not providing a stable comparison environment. The user will test later;
the resolution / shader / hook comparison matrix was stopped. No speedup
percentage is claimed from these runs. A run with UI hooks disabled also
changes the view and draw workload, so it is not an isolated measurement of
interpreter overhead.

The host-step sampler concentrated on `0x0092F408`, the sprite flush. This
sampler sees interpreted instructions individually but sees compiled runs
only at their boundaries. It is **not a CPU wall-time profile**, and does
not establish that sprite flushing dominates total runtime. The first timed
block counted 1,312,992 interpreted and 278,795,895 recompiled instructions;
instruction coverage is not a measure of time spent.

## Optimization order

### 1. Reduce GPU readbacks and synchronous submissions

The host timing and readback attribution make this the first implementation target.
`raster/hardware.rs::run` ends rendering, downloads surfaces, submits, and
waits before converting the results into guest memory. Although dirty rows
limit what is stored back, the download regions cover whole surfaces.
`surface_rows`, `prepare_read`, `sync_where`, and CPU memory guards can all
reach this path. GSP DMA synchronizes source and destination and then copies
through the CPU as well.

Attribution now distinguishes overlapping surface representations, texture
reads, CPU depth reads, other memory synchronization, fills and full flushes.
The battle log identifies overlapping surfaces and texture reads as the
active causes. Next:

- Download only the rows/tiles needed for the request where possible.
- Combine required downloads instead of repeatedly submitting and waiting.
- Keep compatible surface aliases, transfers and DMA copies on the GPU.
- Avoid retrieving old destination bytes when a proven whole-range overwrite
  supersedes them, while preserving every untouched byte.

Do not disable depth readbacks generally. The game can use them for
visibility, and stale data would change behavior.

### 2. Stop rendering unused outputs

Top-only presentation currently fetches both screens in `zakuro::present`.
More significantly, hiding a screen does not stop the guest from rendering
it. `aim_mouse` replays bottom UI drawing on top, so those elements have
their original pass plus the mirrored pass.

The decomp shows explicit drawing into both eye buffers. Examples:
`0x0060D3EC` submits the top HUD to the right eye and then the left;
`0x00727C24` draws scene layers into both. Showing stereo off in the
presenter does not remove those guest submissions.

Investigate a mono rendering path that skips genuinely unused right-eye
draws, and a battle-only bottom rendering path that retains required text
textures and mirrored elements but omits decoration. Keep UI updates,
commands, input and visibility flags running. Compare visible output and
game state across air, ground, boss, pause and menus before enabling either.
Do not assume this saves half the total frame time.

### 3. Replace interpreter-only hook stops with compiled callbacks

The measured build's `aim_mouse::PLACES` marks 30 addresses for interpretation. This
includes every sprite flush (`0x0092F408`), projection getter (`0x003901C4`),
pass setup, and several containing UI functions. `System::step` checks and
dispatches the hook machinery on each interpreted instruction.

Use recompiler-supported hook points / wrappers so a host callback runs at
the required point and the original body stays compiled. Entry hooks such
as sprite flush are good first candidates; hooks inside a function require
correct register, flag, budget and resume handling. This should not require
hand-rewriting the game's sprite command builder from the Ghidra output.
Measure it after the readback work; low interpreted-instruction coverage
alone neither proves nor disproves a worthwhile saving.

### 4. Measure shader placement and narrow GPU barriers

On a discrete GPU, the renderer normally runs vertex shaders on the GPU.
`ZAKURO_CPU_SHADERS=1` provides an existing comparison using CPU shader
execution. Geometry-shader draws have a CPU path already. Compare both on
the same scene after compilation warms up; no winner was established here.

`hardware.rs::barrier` uses `ALL_COMMANDS` and broad memory read/write masks.
The trace shows thousands of these per 600 batches. Track actual resource
dependencies and narrow stage/access scopes where safe. Removing barriers
without dependency tracking risks incorrect rendering.

### 5. Treat 100+ FPS at normal speed as its own feature

Performance improvements alone will leave normal play capped at 59.83 Hz.
A real high-refresh mode needs coordinated changes to frontend scheduling,
console refresh interrupts, the game's present/update pacing, and timestep
conversion. Existing code includes integer update/vblank accounting and
actors whose timestep is expressed as frames/60.

120 Hz is a useful first research target because it is twice 60, but simply
halving one timestep constant does not establish correctness. Check
movement, firing, animation, collision, timers, menus, cutscenes and audio
against equal **elapsed time**, not equal frame counts. Preserve the CPU
clock and DSP sample timing. The existing interpolation path targets one
picture between a 30 FPS game's frames at the console's normal refresh; it
does not already provide arbitrary 100/120 Hz gameplay.

**Spike: native 120 Hz (2026-11-03, uncommitted worktree
`/home/pmill/icarus-high-refresh`, branch `prototype/high-refresh`).** A
prototype IPS (`scripts/make_120fps_ips_PROTOTYPE.py`: 0x0048A7E4 rate 120,
0x0048A7EC swap 1, 0x0091041C no parity, 0x00828E8C an update a vblank,
0x00828E30 0.5) with `ZAKURO_REFRESH_HZ=120` (half the cycles a refresh, CPU
clock and DSP sample clock unchanged) ran 180 refreshes against the 60 FPS
patch's 90 over the same 403.3M cycles from `trace/s7.state`,
`trace/ui-probe/hewdraw-blue.state` and `trace/ui-probe/boss-fight.state`.
The battle held: 180/180 refreshes ran one update and one present (vsync
manager +0xB0 = 1 every refresh, leftover 0), its camera view matched to
0.029 units, the pause menu 0.5 s after START was pixel-identical (mean
0.02/255), the dialogue was within one frame of the 60 FPS run, and the
boss's bottom HUD was identical (max 1/255). The guest sound library still
advances once per game update: the WAV keeps the same 49,280 samples but its
content diverges from sample 483 (DSP service `dsp:` at 0x00CB35B4, client
init referenced at 0x0010D274; the per-update sound entry is not found). The
boss scene missed one refresh in 180 as per-second interpreted instructions
rose 1.79x (59.1M to 106.0M). Hands-on on Windows (default overclock 2):
dialogue audio was not in sync with the scene, and frames dropped noticeably.
**Decision: parked.** High Refresh Mode keeps the game at 60 updates a second
and draws pictures in between in the presenter. Native 120 would first need
the sound update found and kept at 60 Hz, and the frame drops profiled.

## Repeating the measurements on Windows

`dist/kiutrace-perf.exe` is built from the current harness. It loads your
installed `%APPDATA%\3dsrecomp\0004000000030100.dll`; the DLL is not bundled.
The runner copies the chosen state to a fresh temporary directory and keeps
all benchmark output and any guest save writes there. It restores its
environment changes afterward.

Close the game and other GPU-heavy work before comparing. Use the same
battle state and power mode. Run each command separately:

```powershell
.\scripts\benchmark.ps1 -Rom 'C:\path\game.cci' -State '.\trace\states\1.state'
.\scripts\benchmark.ps1 -Rom 'C:\path\game.cci' -State '.\trace\states\1.state' -Scale 1
.\scripts\benchmark.ps1 -Rom 'C:\path\game.cci' -State '.\trace\states\1.state' -Scale 5
.\scripts\benchmark.ps1 -Rom 'C:\path\game.cci' -State '.\trace\states\1.state' -CpuShaders
.\scripts\benchmark.ps1 -Rom 'C:\path\game.cci' -State '.\trace\states\1.state' -NoHooks
.\scripts\benchmark.ps1 -Rom 'C:\path\game.cci' -State '.\trace\states\1.state' -HostProfile
```

Each run warms up for 90 refreshes, times 300, then reloads and repeats.
`-HostProfile` reports host wall time per execution path, with GPU command
lists shown separately as a nested measurement. `-GpuProfile` enables GPU
attribution without host-step timing; plain benchmarks leave both forms of
instrumentation off. GPU timing logs report
readback causes, new download bytes, dirty guest bytes, buffer addresses and
CPU conversion/store time. Those readback summaries cover approximately
600 completed GPU batches, independently of the benchmark blocks.
Confirm the log says NVIDIA, hardware rendering, and direct presentation.
Results are uncapped emulation throughput, not displayed FPS or proof that
the game generates a unique picture every refresh. Repeat on more than one
scene; state 1 alone cannot establish the worst case.

Harness additions: `bench`, `timed`, `profile`, `checkcode`, optional shared-device
presentation and an unhooked baseline. Fixed `peek` returning zero for
read-only executable memory, and added a regression test. `checkcode` is
necessary after restoring a state's patched code: state loading currently
does not itself recheck the executable's compiled functions.

Validation: Linux release build and both regression tests passed (executable
memory reads, and identical CPU state/cycles with host timing enabled).
Windows release
cross-build and native Vulkan battle run completed. A shortened Windows
runner smoke test also verified temporary-state loading, log capture and
environment restoration. The existing user work
in PLAN.md, FINDINGS.md, and aim_mouse.rs was left intact.

## GPU-local color surface aliases

Source/destination attribution found three recurring transitions in the
Chapter 1 battle. In a representative original 600-batch window:

| Transition | Calls | New downloads | Implementation |
| --- | --- | --- | --- |
| `0x1F000000`, RGBA8, 256×512 → 240×320 | 85 | 42.50 MiB | GPU tile copies |
| `0x1F000000`, RGBA8, 240×320 → 256×512 | 85 | 24.90 MiB | GPU tile copies |
| `0x1F300000`, D24S8 plus three RGBA8 views → RGBA8 128×256 | 86 | 55.51 MiB | Existing CPU fallback |

The compatible color transitions were about 55% of overlap download bytes
in that window. The new attribution includes both views' addresses,
dimensions, formats, layout, dirty/excluded byte ranges, requested rows,
call counts, successful GPU copies, host time and new download bytes.
It prints the 20 transitions with the most download bytes/time per window.

`zakuro-gpu/src/raster/hardware.rs::copy_alias` copies matching RGBA8 tiles
between images even when the surface widths differ. Each 8×8 tile has the
same guest byte layout; contiguous image-copy regions stop at either
surface's tile-row boundary. At scales above 1× the copies go through the
existing native-resolution images, using the same nearest sampling as the
old download/upload path. This deliberately preserves the old rendering
result, including its sampling of scaled pixels.

The source retains responsibility for dirty bytes outside the destination.
An `excluded` byte range prevents it from writing back bytes transferred to
the destination, even when the boundary splits a wider surface's tile row.
Writeback preserves those bytes from guest memory, making the final result
independent of which owner is synchronized first. Draws reclaim their tile
rows; fills and state restores clear or supersede ownership appropriately.
Texture lookup excludes stale aliases, destination copies advance image
generations, and CPU write guards remain in force. Surface lookup rounds
tiled requests to complete tile rows so a narrow scissor cannot miss the
owner of other pixels in the same tile.

The path requires one compatible source, unchanged guest-memory shadows,
and no third writer anywhere in the destination. Partial tiles, other pixel
formats, depth/color reinterpretation, pending partial fills, scattered
source memory, ambiguous writers and frame interpolation retain the CPU
fallback. CPU depth synchronization is unchanged. No new shader or dependency
is required.

The path is enabled by default. Set `ZAKURO_GPU_ALIAS_COPY=0` to disable it.
The Windows benchmark runner exposes this as `-NoAliasCopies` and restores
the prior environment afterwards:

```powershell
.\scripts\benchmark.ps1 -Rom 'C:\path\game.cci' -State '.\trace\states\1.state' -NoAliasCopies
.\scripts\benchmark.ps1 -Rom 'C:\path\game.cci' -State '.\trace\states\1.state'
.\scripts\benchmark.ps1 -Rom 'C:\path\game.cci' -State '.\trace\states\1.state' -GpuProfile
```

Use plain runs for performance comparisons and `-GpuProfile` to inspect why
they changed. The optimization removes submissions, so subsequent 600-batch
windows cover more rendering cycles; their download/wait totals cannot be
compared directly with the old windows. The two targeted transitions report
zero new downloads in the optimized NVIDIA profile. The mixed depth/color
transition still downloads through the fallback.

### Alias copy comparison, 15:35 EDT

Six 300-refresh blocks per setting at 3×, state 1, alternating between
settings across three runs; instruction totals identical throughout:

| Setting | Blocks (ms/refresh) | Median | Mean |
| --- | --- | --- | --- |
| `-NoAliasCopies` | 20.16, 18.88, 30.49, 22.56, 18.85, 17.83 | 19.52 | 21.46 |
| Plain (alias copies on) | 15.58, 17.09, 17.26, 18.38, 15.29, 14.76 | 16.34 | 16.39 |

Median improvement about 16% (≈61 refreshes/s against ≈51). The noise
is large on this laptop: the 30.49 ms block is an outlier, and each setting's
fastest and slowest blocks are 25% or more apart. The ranges barely overlap,
so the direction is clear but the size is approximate. Still well short of
100 refreshes/s.

## Glow texture and depth-memory reuse, 16:45 EDT

After the alias copies, the battle still had two readbacks every frame:

- **Texture read, `0x1F080000`:** the frame is drawn 256×512 and sampled
  as a 240×400 RGBA8 texture for the glow. `alias_texture` gives the texture
  a surface of its own width through the same GPU tile copy, then the
  existing drawn-texture path copies from it. The glow now samples the
  scaled frame: top screens differ from the readback path by at most 7/255
  (mean ≈1) in states 1, 2 and 5; states 3 and 4 and every bottom screen
  are identical.
- **Overlapping surfaces, `0x1F300000`:** the glow reuses the start of the
  256×512 depth buffer for 64×64, 32×32 and 16×16 targets, then a 128×256
  target that overlaps the depth's dirty rows. That 128×256 draw is one
  indexed quad over the whole buffer, blending with destination factor zero.
  A draw proven to cover its whole buffer (two unclipped, unculled triangles
  corner to corner, split along a diagonal) that writes all channels with no
  destination-reading blend, logic op, alpha, depth or stencil test now takes
  the bytes from other surfaces (`relinquish`) instead of writing them back.
  Draws of six vertices or fewer are decoded on the CPU so the check can run
  their vertex shader; the GPU still shades them. Screenshots match the
  glow-texture build exactly in all five states.

With both, the GPU profile reports no readbacks and 0.0 ms of CPU
conversion in this scene. State 1 at 3×, alternating builds, same instruction
totals:

| Build | ms/refresh (six 300-refresh blocks) |
| --- | --- |
| `3f0f7fc` (alias copies) | 22.97, 24.65, 22.24, 24.78, 21.91, 24.39 |
| `b8ea910` (this change) | 9.02, 8.17, 9.16, 8.29, 8.43, 8.32 |

About 8.6 ms/refresh, or 116 uncapped refreshes/s, against about 23.5 ms on
the same machine in the same session. The machine was slower than in the
earlier comparison, so compare ratios, not absolute times. This is emulation
throughput at the console's game rate, not 100+ distinct game frames per
second; that still needs the timing work in section 5.

### Correction: alias copies at scale, 17:20 EDT

The alias copies went through the console-resolution images. Once the glow's
240-wide surface owned the frame and the frame copied it back, land battles
dropped to 1× (`37f4ea6` fixes it). Tiles move whole, so the scaled images now
copy directly, each tile scale× larger. State 1 is now pixel-identical to
`3f0f7fc`, so the glow difference noted above is gone; 8.3–8.8 ms/refresh.
Screenshots taken with `KIUTRACE_DIRECT` read the picture back through guest
memory and show 1× in every build; check sharpness without it.

## Bloom footprint and repeating-grain minification, 31 October 2026

The curved screen-door pattern is introduced **before bloom**. A 36-vertex
cube draw in the air battle samples the 128×128 ETC1 texture at `0x1F46A800`
with texture coordinates in the thousands, using mirrored repeat and linear
minification. The old Vulkan texture upload provided only mip 0. Both the
software and hardware rasterizers showed the resulting moiré; framebuffer
alias copies on/off and CPU vertex shading did not remove it. Per-draw
software captures locate its first appearance at draw 27 in the current
Windows slot 1. Adding a tiny tolerance to final color truncation did not
help and was reverted.

The hardware renderer now uploads decoded guest textures with the same
alpha-weighted mip builder used for texture packs. Linear minification can
select these levels with an unbiased LOD. Magnification and minification
use their separate register bits; point magnification and base-level texture
data stay intact. Texture-pack sharpening retains its existing bias.
This fixes minified grain without a final-frame blur. Software rasterization
still uses its existing base-level sampling.

Linearly filtered framebuffer effects that resize their input, without a
depth/stencil attachment, now average scaled source samples within each guest
pixel before sampling the resulting native-sized copy. This restores the
blur footprint of the small glow buffers while retaining the original taps,
combiner constants and blending. Native and scaled copies have separate
cache keys, including separate twin caches, and each follows the source's
generation. Nearest-filtered scene copies keep scaled detail. Existing tile
alias ownership and guest-memory writeback are unchanged. The existing
transfer compute shader performs the averaging; no shader changes remain.

Reproduction scripts, per-draw diagnostics, before/after captures and build/
test logs are in `trace/glow-fix/`. The diagnostic draw dumps and tracing code
were removed from the renderer after isolating the offending draw.

Validation: the Vulkan GPU suite passed 177 tests (2 existing tests ignored).
Final focused checks passed the grain/magnification regression, the effect
copy regression extended to both real and twin images, and all 11 texture
pack tests. Captures cover 1×, 3× and 6× at 5:3 and 16:9 for sky and ground,
plus pause/dialog at 3× and alias-copy on/off comparisons. Interpolation was
toggled successfully on the shared-device path; the saved air scene did not
select an intermediate frame at the sampled refreshes, so the synthetic
twin-image regression supplies the direct check of that copy path.

The local Linux software-Vulkan timing comparison was too noisy for a
performance conclusion: before 772/950 ms per refresh, after 1491/565 ms per
refresh, during concurrent compilation. The two 60-refresh blocks execute
identical interpreted/recompiled instruction counts before and after. These
are llvmpipe measurements, not NVIDIA performance estimates. Native Windows
visual/performance verification remains a manual check.

Windows release cross-build completed successfully and was copied to
`dist/zakuro-kiu-mouse.exe` (SHA-256
`cb0acaf05f4bcc416187eff0b0eeeb245f8d58261ddebd93d4b83e33d8304187`).
The prior executable is retained as `dist/zakuro-kiu-mouse-before-glow.exe`.
The final native smoke capture matches the verified fixed capture byte for
byte on both screens.
