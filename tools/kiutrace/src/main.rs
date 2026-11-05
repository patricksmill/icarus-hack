//! Runs Kid Icarus: Uprising headless in Zakuro's interpreter from a script
//! of commands, to find where input reaches the game: screenshots to see
//! where it is, scripted buttons and touches, a scan for aim objects by
//! vtable, and a per-instruction trace of what writes a watched range.
//!
//! usage: kiutrace <rom> <script> [out dir]
//!
//! script commands, one per line, # for comments:
//!   run <frames>                  run with the current input
//!   bench <frames>                time emulation without presentation; warm up with run first
//!   timed <frames>                bench with host wall times per execution path (adds timing overhead)
//!   profile <frames>              sample host-step PCs (biased toward interpreted code)
//!   hold <buttons...>             hold buttons (A B X Y L R START SELECT UP DOWN LEFT RIGHT ZL ZR), none releases
//!   touch <x> <y> / untouch       touch the bottom screen at a pixel, or let go
//!   circle <x> <y>                circle pad, -1..1
//!   drag <x0> <y0> <x1> <y1> <frames>   touch and slide across frames, then let go
//!   shot <name>                   write both screens as out/<name>_top.ppm and _bottom.ppm
//!   shotscaled <name>             the same at the hardware renderer's scale, KIUTRACE_SCALE=<n>
//!                                 also writes <name>_right.ppm when the game draws in 3D
//!   stereo <0..1>                 set the console's 3D slider (0 is off)
//!   save <file> / load <file>     a Zakuro saved state
//!   scan                          list objects whose first word is an aim vtable
//!   peek <addr> <count>           print count words at addr as hex and float
//!   parts <object>                print the active menu object's retained native descriptor kinds and words
//!   expectword <addr> <value>      assert a native game-state word, such as the keyboard Cursor's Control hash
//!   poke <addr> <value>           write a word, a float when it has a point
//!   code <addr> <word>            write a word of code, past the page's permissions
//!   checkcode                    recheck the library after loading a state or patching code
//!   dump <addr> <len> <name>      write len bytes of memory at addr as out/<name>.bin
//!   watch <addr> <len>            add a range to the trace
//!   trace <frames>                run instruction by instruction, printing each write to a watched range
//!   aim <dx> <dy> <frames>        run with Zakuro's own mouse aiming captured, moving dx, dy a frame
//!   power                         press the power key as Zakuro does: in a battle, uses the current power
//!   mirror [0|1]                  mirror battle UI, dialogue images and boss cards on top (default on)
//!   portraits <scale>             draw the portraits on the top screen too at scale, with Zakuro's hook
//!   narrtop                       draw the narration on the top screen too, with Zakuro's hook
//!   pausetop                      draw the pause menu on the top screen too, with Zakuro's hook
//!   menutop [0|1]                 draw scripted menus on the top screen (default on)
//!   mhover <x> <y>                hover over the top screen, in its 400 by 240 pixels
//!   mfrom <x> <y> [button hash]   hover there using an original bottom-screen point
//!   mdrag <x0> <y0> <x1> <y1> <frames> drag those native points through the top-screen mapping
//!   layer <index> <x> <y> <w> <h> move/resize a Scripted Menu layer through the editor
//!   visible <index> <0|1>         hide/show a Scripted Menu editor item, without changing its geometry
//!   pausevisible <span|center|left|right|corner> <0|1>  hide/show one Pause rule group
//!   navlink <from> <up|down|left|right> <to|native> set an explicit editor navigation link
//!   cauldron                      draw the Fiend's Cauldron's two screens on the top screen, laid out to fit it
//!   ctouch <x> <y>                touch the Fiend's Cauldron drawn on the top screen at its pixel, x of its 400
//!   cdrag <x0> <y0> <x1> <y1> <frames>  and drag across it
//!   chover <x> <y>                move the mouse over it there, without touching
//!   cauldronrule <role> <to x y w h> <from x y w h> <anchor> <anchor_y>  set one Fiend's Cauldron placement rule:
//!                                 role top, panel, bar, dial, back, moveout or rest; to is the box at its design
//!                                 width as shown, from the part of its screen shown (panel and dial at intensity 0)
//!   cauldronvisible <role> <0|1>  hide/show one Fiend's Cauldron part, drawing and touches both
//!   hover <x> <y> / unhover       move the mouse over the bottom screen's pixel, or off it, with
//!                                 Zakuro's menu hover on: menus light what is under it, keys give them back
//!   click                         click the mouse where it hovers: a menu outside battle outlines and uses
//!                                 the part under it, as A would, or prints that it did not take it
//!   press <frames> / release      hold the physical mouse button, polling the frontend's click/drag routing
//!                                 each frame, or release it (a short press clicks, a hold or move touches)
//!   wheel <rows> [columns]        scroll the hovered menu; positive down/right (columns is Shift+wheel)
//!   shown                         print what Zakuro's hooks see shown: a battle, the pause menu, the Fiend's Cauldron
//!   interpolation [0|1]           inspect or toggle frame interpolation (requires KIUTRACE_DIRECT=1)
//!   wide <aspect>                 widen the top screen to aspect (width / height), with Zakuro's hooks
//!   hud <scale>                   draw the top screen's HUD and reticle at scale, with Zakuro's hook
//!   calls <addr> <frames> [max]   print r0..r3, lr, sp, s0..s3 and r0's first words each time addr runs, up to max (8) a frame;
//!                                 addr can be several, a,b,c, printed in the order they run
//!   ran <frames> <file>           write each function that ran in every one of the frames to out/<file>, and in any
//!                                 of them to out/<file>.any, by analysis/functions.txt
//!   entered <value> <frames>      print each function entered with value in r0, once each (run from the project, for analysis/functions.txt)
//!   quads <frames>                print each 2D sprite batch flushed (0x0092F408): caller, and each quad's box
//!   narr <x> <y> <frames>         run, also drawing the narration line on the top screen at x, y
//!   call <function> <r0> <r1> [r2]  run a frame, the game calling function with r0..r2 where the menus outside
//!                                 battle's touch has none (0x00A391E4), as the mouse's hooks call a menu's own code
//!   mouse <dx> <dy> <frames>      run, adding a mouse movement of dx, dy reticle pixels per frame to
//!                                 the aim delta where 0x00B12218 returns, as the override will
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use zakuro_common::Screen;
use zakuro_core::loader;
use zakuro_core::services::hid::{InputState, PadState};
use zakuro_core::{Config, FrameOutcome, StepOutcome, System};

/// the vtables of the three aim controller classes in the USA build.
const AIM_VTABLES: [(u32, &str); 3] = [(0x00CEDA50, "aim (stub input)"), (0x00CF1B40, "aim (0x76E854)"), (0x00CF1B94, "aim (0x76EBF0)")];

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .target(env_logger::Target::Stdout)
        .init();
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: kiutrace <rom> <script> [out dir]");
        std::process::exit(2);
    }
    let out = PathBuf::from(args.get(3).map(String::as_str).unwrap_or("kiutrace-out"));
    std::fs::create_dir_all(&out).expect("create the output directory");
    let script = std::fs::read_to_string(&args[2]).expect("read the script");

    let new3ds = std::env::var_os("KIUTRACE_NEW3DS").is_some();
    // KIUTRACE_LIB=<library> runs the recompiled code, but for the functions
    // Zakuro's mouse aiming stops in
    let recompiled = std::env::var_os("KIUTRACE_LIB").map(PathBuf::from);
    // KIUTRACE_SCALE=<n> draws on the host's GPU at n times the resolution,
    // as the frontend's hardware renderer does
    let scale: u32 = std::env::var("KIUTRACE_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    // Match the frontend's shared-device path; KIUTRACE_DIRECT also asks for
    // the screen handles each refresh, without a window or display wait.
    let direct = std::env::var_os("KIUTRACE_DIRECT").is_some();
    let device = direct.then(|| std::sync::Arc::new(zakuro_gpu::SharedDevice::new().expect("create the shared GPU device")));
    let data_dir = std::env::var_os("KIUTRACE_DATA_DIR").map(PathBuf::from);
    let config = Config { hardware_renderer: scale > 0, resolution: scale.max(1), device, find_recompiled: false, new3ds, recompiled, data_dir, ..Config::default() };
    let mut system = loader::load(&args[1], config).unwrap_or_else(|error| {
        eprintln!("could not load {}: {error}", args[1]);
        std::process::exit(1);
    });
    // A comparison with the unhooked library must use a script with no hooks.
    if let Some(library) = system.recompiled.as_mut().filter(|_| std::env::var_os("KIUTRACE_NO_HOOKS").is_none()) {
        for address in zakuro_core::aim_mouse::PLACES {
            println!("interpreting at 0x{address:08X}: {} functions", library.interpret_at(address));
        }
    }
    let mut input = InputState::default();
    let mut watches: Vec<(u32, u32)> = Vec::new();
    let mut frame: u64 = 0;

    for (number, line) in script.lines().enumerate() {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let num = |i: usize| -> u32 { parse(words.get(i).copied().unwrap_or("0")) };
        let float = |i: usize| -> f32 { words.get(i).and_then(|w| w.parse().ok()).unwrap_or(0.0) };
        println!("[{frame}] {line}");
        match words[0] {
            "run" | "bench" | "profile" | "timed" => {
                if words[0] == "profile" {
                    system.enable_profiler();
                }
                if words[0] == "timed" {
                    system.host_timings = Some(zakuro_core::HostTimings::default());
                }
                let start = std::time::Instant::now();
                let before = (system.interpreted_instructions, system.recompiled_instructions);
                for _ in 0..num(1) {
                    if !run_frame(&mut system, input, &mut frame) {
                        return;
                    }
                    if direct {
                        for screen in [Screen::Top, Screen::Bottom] {
                            std::hint::black_box(system.gpu_screen(screen));
                        }
                    }
                }
                let seconds = start.elapsed().as_secs_f64();
                if words[0] != "run" && num(1) > 0 {
                    let interpreted = system.interpreted_instructions - before.0;
                    let recompiled = system.recompiled_instructions - before.1;
                    println!(
                        "  {} frames in {seconds:.3}s: {:.2} ms/frame, {:.1} emulated refreshes/s; {interpreted} interpreted, {recompiled} recompiled instructions; GPU {}",
                        num(1), seconds * 1000.0 / num(1) as f64, num(1) as f64 / seconds, system.gpu.draws_on_gpu()
                    );
                }
                if words[0] == "timed" {
                    let timing = system.host_timings.take().expect("host timing was enabled");
                    if num(1) > 0 {
                        print_host_timings(&timing, num(1), seconds);
                    }
                }
                if words[0] == "profile" {
                    for (thread, pc, hits) in system.hot_spots(30) {
                        println!("  {pc:08X} {hits:8} {thread}");
                    }
                    system.profile = None;
                }
            }
            "hold" => input.buttons = words[1..].iter().map(|name| button(name)).fold(PadState::empty(), |a, b| a | b),
            "touch" => input.touch = Some((num(1) as u16, num(2) as u16)),
            "untouch" => input.touch = None,
            "circle" => (input.circle_x, input.circle_y) = (float(1), float(2)),
            "drag" => {
                let (x0, y0, x1, y1, frames) = (float(1), float(2), float(3), float(4), num(5).max(1));
                for i in 0..=frames {
                    let t = i as f32 / frames as f32;
                    input.touch = Some(((x0 + (x1 - x0) * t) as u16, (y0 + (y1 - y0) * t) as u16));
                    if !run_frame(&mut system, input, &mut frame) {
                        return;
                    }
                }
                input.touch = None;
            }
            "shot" => {
                for (screen, suffix) in [(Screen::Top, "top"), (Screen::Bottom, "bottom")] {
                    let path = out.join(format!("{}_{suffix}.ppm", words[1]));
                    write_ppm(&path, screen.width(), screen.height(), &system.read_screen(screen));
                }
            }
            "shotscaled" => {
                for (screen, suffix) in [(Screen::Top, "top"), (Screen::Bottom, "bottom")] {
                    let path = out.join(format!("{}_{suffix}.ppm", words[1]));
                    let (image, width, height) = system.read_screen_scaled(screen);
                    write_ppm(&path, width, height, &image);
                }
                if let Some((image, width, height)) = system.read_right_eye_scaled() {
                    write_ppm(&out.join(format!("{}_right.ppm", words[1])), width, height, &image);
                }
            }
            "stereo" => system.set_slider_3d(float(1).clamp(0.0, 1.0)),
            "save" => std::fs::write(words[1], system.save_state().expect("save the state")).expect("write the state"),
            "load" => system.load_state(&std::fs::read(words[1]).expect("read the state")).expect("load the state"),
            "scan" => scan(&mut system),
            "peek" => {
                for i in 0..num(2) {
                    let address = num(1) + i * 4;
                    let word = read32(&mut system, address);
                    println!("  {address:08X}  {word:08X}  {}", f32::from_bits(word));
                }
            }
            "watch" => watches.push((num(1), num(2))),
            "dump" => {
                let mut bytes = vec![0u8; num(2) as usize];
                system.memory.read_bytes(num(1), &mut bytes);
                let _ = std::fs::write(out.join(format!("{}.bin", words[3])), &bytes);
            }
            "code" => {
                if !system.memory.write32_privileged(num(1), num(2)) {
                    println!("  could not write {:08X}", num(1));
                }
            }
            "checkcode" => {
                if let Some(library) = &mut system.recompiled {
                    let stale = library.check(&mut system.memory);
                    println!("  {stale} functions differ from the recompiled code");
                }
            }
            "poke" => {
                let value = if words[2].contains('.') { float(2).to_bits() } else { num(2) };
                system.memory.write_bytes(num(1), &value.to_le_bytes());
            }
            "trace" => {
                for _ in 0..num(1) {
                    if !trace_frame(&mut system, input, &watches, &mut frame) {
                        return;
                    }
                }
            }
            "aim" => {
                system.aim_mouse.get_or_insert_with(Default::default).captured = true;
                for _ in 0..num(3) {
                    if let Some(aim_mouse) = &mut system.aim_mouse {
                        aim_mouse.add(float(1), float(2));
                    }
                    if !run_frame(&mut system, input, &mut frame) {
                        return;
                    }
                }
            }
            "power" => {
                let frame = system.frames;
                let aim_mouse = system.aim_mouse.get_or_insert_with(Default::default);
                let in_battle = aim_mouse.in_battle(frame);
                if in_battle {
                    aim_mouse.use_power(frame);
                }
                println!("  [{frame}] power key: {}", if in_battle { "uses the current power" } else { "ZL, not in a battle" });
            }
            "overclock" => system.cpu.overclock = num(1).max(1),
            "mirror" => system.aim_mouse.get_or_insert_with(Default::default).mirror_bottom = words.len() == 1 || num(1) != 0,
            "portraits" => system.aim_mouse.get_or_insert_with(Default::default).portrait_scale = float(1),
            "narrtop" => system.aim_mouse.get_or_insert_with(Default::default).narration_on_top = true,
            "pausetop" => system.aim_mouse.get_or_insert_with(Default::default).pause_on_top = true,
            "menutop" => system.aim_mouse.get_or_insert_with(Default::default).menus_on_top = words.len() == 1 || num(1) != 0,
            "mhover" | "mfrom" => {
                let frame = system.frames;
                let mouse = system.aim_mouse.get_or_insert_with(Default::default);
                let point = if words[0] == "mfrom" { mouse.menus_top_point(frame, float(1), float(2), (words.len() > 3).then(|| num(3))) } else { Some((float(1), float(2))) };
                let pointer = point.and_then(|(x, y)| mouse.menus_top_touch(frame, x, y, false));
                mouse.menu_hover = true;
                mouse.hover(pointer);
                println!("  top {point:?} -> bottom {pointer:?}");
            }
            "mdragtop" => {
                let (start, end, count) = ([float(1), float(2)], [float(3), float(4)], num(5).max(1));
                // Drive host pixels directly, as the frontend does. Do not
                // reverse-map a native path through changing menu parts.
                for _ in 0..12 {
                    let mouse = system.aim_mouse.as_mut().expect("menutop before mdragtop");
                    let pointer = mouse.menus_top_touch(system.frames, start[0], start[1], false);
                    mouse.menu_hover = true;
                    mouse.hover(pointer);
                    if !run_frame(&mut system, input, &mut frame) { return; }
                }
                for step in 0..=count {
                    let mouse = system.aim_mouse.as_mut().unwrap();
                    let t = step as f32 / count as f32;
                    let point = [start[0] + (end[0] - start[0]) * t, start[1] + (end[1] - start[1]) * t];
                    let pointer = mouse.menus_top_touch(system.frames, point[0], point[1], true);
                    mouse.hover(pointer);
                    input.touch = mouse.menu_touch(system.frames, true).map(|(x, y)| (x.clamp(0.0, 319.0) as u16, y.clamp(0.0, 239.0) as u16));
                    println!("  drag top {point:?}, mapped {pointer:?}, touch {:?}", input.touch);
                    if !run_frame(&mut system, input, &mut frame) { return; }
                }
                for _ in 0..30 {
                    let mouse = system.aim_mouse.as_mut().unwrap();
                    let pointer = mouse.menus_top_touch(system.frames, end[0], end[1], true);
                    mouse.hover(pointer);
                    input.touch = mouse.menu_touch(system.frames, true).map(|(x, y)| (x.clamp(0.0, 319.0) as u16, y.clamp(0.0, 239.0) as u16));
                    if !run_frame(&mut system, input, &mut frame) { return; }
                }
                let mouse = system.aim_mouse.as_mut().unwrap();
                let pointer = mouse.menus_top_touch(system.frames, end[0], end[1], false);
                mouse.hover(pointer);
                mouse.menu_touch(system.frames, false);
                input.touch = None;
            }
            "mdrag" => {
                let (start, end, count) = ([float(1), float(2)], [float(3), float(4)], num(5).max(1));
                for step in 0..=count {
                    let mouse = system.aim_mouse.as_mut().expect("menutop before mdrag");
                    let t = step as f32 / count as f32;
                    let (x, y) = mouse.menus_top_point(system.frames, start[0] + (end[0] - start[0]) * t, start[1] + (end[1] - start[1]) * t, None).expect("visible top-screen control");
                    let pointer = mouse.menus_top_touch(system.frames, x, y, true);
                    mouse.hover(pointer);
                    input.touch = mouse.menu_touch(system.frames, true).map(|(x, y)| (x.clamp(0.0, 319.0) as u16, y.clamp(0.0, 239.0) as u16));
                    if !run_frame(&mut system, input, &mut frame) { return; }
                }
                let mouse = system.aim_mouse.as_mut().unwrap();
                mouse.menus_top_touch(system.frames, 0.0, 0.0, false);
                mouse.menu_touch(system.frames, false);
                input.touch = None;
            }
            "cauldron" => system.aim_mouse.get_or_insert_with(Default::default).cauldron_on_top = true,
            "ctouch" | "cdrag" | "chover" => {
                // the top screen's pixel, x of its 400, as the frontend gives it
                let mut path = vec![(float(1), float(2))];
                if words[0] == "cdrag" {
                    let steps = num(5).max(1);
                    path = (0..=steps).map(|i| (float(1) + (float(3) - float(1)) * i as f32 / steps as f32, float(2) + (float(4) - float(2)) * i as f32 / steps as f32)).collect();
                }
                for (x, y) in path {
                    let frames = system.frames;
                    let at = system.aim_mouse.as_mut().and_then(|aim_mouse| aim_mouse.top_screen_touch(frames, x, y, words[0] != "chover"));
                    println!("  top {x} {y} -> bottom {at:?}");
                    input.touch = at.filter(|_| words[0] != "chover").map(|(x, y)| (x as u16, y as u16));
                    for _ in 0..if words[0] == "cdrag" { 1 } else { 6 } {
                        if !run_frame(&mut system, input, &mut frame) {
                            return;
                        }
                    }
                }
                input.touch = None;
                let frames = system.frames;
                if let Some(aim_mouse) = system.aim_mouse.as_mut() {
                    aim_mouse.top_screen_touch(frames, 0.0, 0.0, false);
                }
            }
            "cauldronrule" | "cauldronvisible" => {
                use zakuro_core::aim_mouse::CauldronRole;
                let role = words.get(1).and_then(|name| CauldronRole::from_name(name))
                    .unwrap_or_else(|| panic!("{} requires a Cauldron role: top, panel, bar, dial, back, moveout, rest or ring", words[0]));
                let mouse = system.aim_mouse.get_or_insert_with(Default::default);
                let mut rules = mouse.cauldron_rules();
                let rule = rules.rule_mut(role);
                if words[0] == "cauldronvisible" {
                    rule.visible = num(2) != 0;
                } else {
                    rule.to = [float(2), float(3), float(4), float(5)];
                    rule.from = [float(6), float(7), float(8), float(9)];
                    (rule.anchor, rule.anchor_y) = (float(10), float(11));
                }
                mouse.set_cauldron_rules(rules);
            }
            "hover" | "unhover" => {
                let aim_mouse = system.aim_mouse.get_or_insert_with(Default::default);
                aim_mouse.menu_hover = true;
                aim_mouse.hover((words[0] == "hover").then(|| (float(1), float(2))));
            }
            "click" => {
                let frames = system.frames;
                let aim_mouse = system.aim_mouse.get_or_insert_with(Default::default);
                println!("  click taken by the menu: {}", aim_mouse.menu_click(frames));
            }
            "press" | "release" => {
                let pressed = words[0] == "press";
                for _ in 0..if pressed { num(1).max(1) } else { 1 } {
                    let mouse = system.aim_mouse.get_or_insert_with(Default::default);
                    mouse.menu_hover = true;
                    input.touch = mouse.menu_touch(system.frames, pressed).map(|(x, y)| (x as u16, y as u16));
                    if pressed && !run_frame(&mut system, input, &mut frame) {
                        return;
                    }
                }
            }
            "wheel" => {
                let mouse = system.aim_mouse.get_or_insert_with(Default::default);
                println!("  wheel taken by the menu: {}", mouse.scroll_menu(system.frames, [float(2), float(1)]));
            }
            "shown" => {
                let aim_mouse = system.aim_mouse.get_or_insert_with(Default::default);
                let frames = system.frames;
                println!("battle {} pause {} cauldron {} menus {} part {:08X?}", aim_mouse.in_battle(frames), aim_mouse.pause_menu_shown(frames), aim_mouse.cauldron_shown(frames), aim_mouse.menus_on_top_shown(frames), aim_mouse.hovered_menu_part());
            }
            "parts" => {
                let owner = read32(&mut system, num(1) + 0x30);
                let head = owner + 0x1A8;
                let mut pending = vec![read32(&mut system, owner + 0x1AC)];
                let mut seen = std::collections::HashSet::new();
                while let Some(node) = pending.pop() {
                    if node == 0 || node == head || !seen.insert(node) || seen.len() > 128 { continue; }
                    pending.extend([read32(&mut system, node + 8), read32(&mut system, node + 12)]);
                    let kind = read32(&mut system, node + 0x14);
                    let kept = read32(&mut system, node);
                    let descriptor = read32(&mut system, kept + 0x1C);
                    if descriptor == 0 || !system.memory.is_readable(descriptor, 0x100) { continue; }
                    let words: Vec<_> = (0..64).map(|i| format!("{:08X}", read32(&mut system, descriptor + i * 4))).collect();
                    println!("  kind {kind:08X} descriptor {descriptor:08X}: {}", words.join(" "));
                }
            }
            "layers" => {
                let mouse = system.aim_mouse.get_or_insert_with(Default::default);
                for (name, to) in mouse.editor_layers() {
                    println!("  {name}: {to:?}");
                }
            }
            "layer" => {
                let mouse = system.aim_mouse.as_mut().expect("menutop before layer");
                let index = num(1) as usize;
                assert!(index < mouse.editor_layers().len(), "visible editor layer");
                mouse.set_editor_layer(index, [float(2), float(3), float(4), float(5)]);
            }
            "visible" => {
                let mouse = system.aim_mouse.as_mut().expect("menutop before visible");
                let index = num(1) as usize;
                assert!(index < mouse.editor_layers().len(), "editor item");
                mouse.set_editor_layer_visible(index, num(2) != 0);
            }
            "pausevisible" => {
                let mouse = system.aim_mouse.get_or_insert_with(Default::default);
                let mut rules = mouse.pause_rules();
                let rule = match words.get(1).copied() {
                    Some("span") => &mut rules.span,
                    Some("center") => &mut rules.center,
                    Some("left") => &mut rules.left,
                    Some("right") => &mut rules.right,
                    Some("corner") => &mut rules.corner,
                    _ => panic!("pausevisible requires a Pause rule name"),
                };
                rule.visible = num(2) != 0;
                mouse.set_pause_rules(rules);
            }
            "navlink" => {
                use zakuro_core::aim_mouse::NavigationDirection;
                let direction = match words.get(2).copied() {
                    Some("up") => NavigationDirection::Up,
                    Some("down") => NavigationDirection::Down,
                    Some("left") => NavigationDirection::Left,
                    Some("right") => NavigationDirection::Right,
                    _ => panic!("navlink direction must be up, down, left or right"),
                };
                let mouse = system.aim_mouse.as_mut().expect("menutop before navlink");
                let controls = mouse.editor_navigation_controls();
                let from = num(1) as usize;
                let to = (words.get(3).copied() != Some("native")).then(|| num(3) as usize);
                assert!(controls.iter().any(|(index, _)| *index == from), "navigation source Control");
                assert!(to.is_none_or(|to| controls.iter().any(|(index, _)| *index == to)), "navigation destination Control");
                mouse.set_editor_navigation_link(from, direction, to);
            }
            "expectword" => {
                let actual = read32(&mut system, num(1));
                assert_eq!(actual, num(2), "native word at {:#010X}", num(1));
            }
            "expectmenu" => {
                let expected = (words[1] != "none").then(|| num(1));
                assert_eq!(system.aim_mouse.as_ref().and_then(|mouse| mouse.hovered_menu_part()), expected, "hovered menu part");
            }
            "interpolation" => {
                if words.len() > 1 {
                    println!("  enabled: {}", system.set_interpolation(num(1) != 0));
                }
                println!("  between: {}, matches: {:?}", system.shows_between(), system.interpolation_matches());
            }
            "wide" => system.aim_mouse.get_or_insert_with(Default::default).widen = float(1) / (5.0 / 3.0),
            "hud" => system.aim_mouse.get_or_insert_with(Default::default).hud_scale = float(1),
            "calls" => {
                let addresses: Vec<u32> = words[1].split(',').map(parse).collect();
                if let Some(library) = system.recompiled.as_mut() {
                    for &address in &addresses {
                        println!("  interpreting {address:08X}: {} functions", library.interpret_at(address));
                    }
                }
                for _ in 0..num(2) {
                    let max = if words.len() > 3 { num(3) } else { 8 };
                    if !calls_frame(&mut system, input, &addresses, max, &mut frame) {
                        return;
                    }
                }
            }
            "ran" => {
                let list = std::fs::read_to_string("analysis/functions.txt").expect("read analysis/functions.txt");
                let functions: std::collections::HashSet<u32> = list.lines().filter_map(|line| u32::from_str_radix(line.split_whitespace().next()?, 16).ok()).collect();
                let mut every: Option<std::collections::BTreeSet<u32>> = None;
                let mut any = std::collections::BTreeSet::new();
                for _ in 0..num(1) {
                    let mut this = std::collections::BTreeSet::new();
                    system.set_input(input);
                    zakuro_core::services::hid::update(&mut system, input);
                    let deadline = system.cpu.cycles + zakuro_core::CYCLES_PER_FRAME;
                    while system.cpu.cycles < deadline {
                        let pc = system.cpu.regs[15];
                        if functions.contains(&pc) {
                            this.insert(pc);
                        }
                        if !matches!(system.step(Some(deadline)), StepOutcome::Ran) {
                            println!("stopped");
                            return;
                        }
                    }
                    frame += 1;
                    any.extend(this.iter().copied());
                    every = Some(match every {
                        Some(every) => every.intersection(&this).copied().collect(),
                        None => this,
                    });
                }
                let text = |set: &std::collections::BTreeSet<u32>| set.iter().map(|pc| format!("{pc:08X}\n")).collect::<String>();
                std::fs::write(out.join(words[2]), text(&every.unwrap_or_default())).expect("write the functions");
                std::fs::write(out.join(format!("{}.any", words[2])), text(&any)).expect("write the functions");
            }
            "entered" => {
                let list = std::fs::read_to_string("analysis/functions.txt").expect("read analysis/functions.txt");
                let functions = list.lines().filter_map(|line| u32::from_str_radix(line.split_whitespace().next()?, 16).ok()).collect();
                let mut seen = std::collections::HashSet::new();
                for _ in 0..num(2) {
                    if !entered_frame(&mut system, input, num(1), &functions, &mut seen, &mut frame) {
                        return;
                    }
                }
            }
            "quads" => {
                for _ in 0..num(1) {
                    if !quads_frame(&mut system, input, &mut frame) {
                        return;
                    }
                }
            }
            "narr" => {
                for _ in 0..num(3) {
                    if !narration_frame(&mut system, input, float(1), float(2), &mut frame) {
                        return;
                    }
                }
            }
            "mouse" => {
                for _ in 0..num(3) {
                    if !mouse_frame(&mut system, input, float(1), float(2), &mut frame) {
                        return;
                    }
                }
            }
            "call" => {
                if !call_frame(&mut system, input, num(1), [num(2), num(3), num(4)], &mut frame) {
                    return;
                }
            }
            other => eprintln!("line {}: unknown command {other}", number + 1),
        }
    }
}

fn print_host_timings(timing: &zakuro_core::HostTimings, frames: u32, seconds: f64) {
    let ms = |span: &zakuro_core::HostSpan| span.elapsed.as_secs_f64() * 1000.0 / frames as f64;
    let mut accounted = 0.0;
    println!("  host timings include GPU waits within each path; instrumentation adds overhead");
    for (name, span) in [
        ("compiled code", &timing.compiled), ("interpreter", &timing.interpreted),
        ("hooks", &timing.hooks), ("audio/devices", &timing.audio), ("frame boundary", &timing.frame),
    ] {
        accounted += ms(span);
        println!("  host {name}: {:.3} ms/frame, {} calls", ms(span), span.calls);
    }
    let mut syscalls: Vec<_> = timing.syscalls.iter().collect();
    syscalls.sort_by_key(|(_, span)| std::cmp::Reverse(span.elapsed));
    for (number, span) in syscalls {
        accounted += ms(span);
        println!("  host SVC 0x{number:02X}: {:.3} ms/frame, {} calls", ms(span), span.calls);
    }
    println!("  host GPU command lists: {:.3} ms/frame, {} calls (nested in paths above)", ms(&timing.gpu_commands), timing.gpu_commands.calls);
    println!("  host remaining scheduler/lookup/screen/timing overhead: {:.3} ms/frame", (seconds * 1000.0 / frames as f64 - accounted).max(0.0));
}

fn parse(word: &str) -> u32 {
    match word.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).unwrap_or(0),
        None => word.parse().unwrap_or(0),
    }
}

fn button(name: &str) -> PadState {
    match name.to_ascii_uppercase().as_str() {
        "A" => PadState::A,
        "B" => PadState::B,
        "X" => PadState::X,
        "Y" => PadState::Y,
        "L" => PadState::L,
        "R" => PadState::R,
        "ZL" => PadState::ZL,
        "ZR" => PadState::ZR,
        "START" => PadState::START,
        "SELECT" => PadState::SELECT,
        "UP" => PadState::UP,
        "DOWN" => PadState::DOWN,
        "LEFT" => PadState::LEFT,
        "RIGHT" => PadState::RIGHT,
        _ => PadState::empty(),
    }
}

fn run_frame(system: &mut System, input: InputState, frame: &mut u64) -> bool {
    system.set_input(input);
    let outcome = system.run_frame();
    *frame += 1;
    if outcome != FrameOutcome::Completed {
        println!("stopped: {outcome:?} at frame {frame}, {}", system.fatal_errors.join("; "));
        return false;
    }
    true
}

/// runs a frame one instruction at a time, printing the registers each
/// time address is about to run, up to max times.
fn calls_frame(system: &mut System, input: InputState, addresses: &[u32], max: u32, frame: &mut u64) -> bool {
    system.set_input(input);
    zakuro_core::services::hid::update(system, input);
    let deadline = system.cpu.cycles + zakuro_core::CYCLES_PER_FRAME;
    let mut count = 0;
    while system.cpu.cycles < deadline {
        let address = system.cpu.regs[15];
        if addresses.contains(&address) && count < max {
            count += 1;
            let r = system.cpu.regs;
            let s: Vec<String> = (0..4).map(|i| format!("{}", f32::from_bits(system.cpu.vfp.regs[i]))).collect();
            let words: Vec<String> = (0..8).map(|i| format!("{:08X}", read32(system, r[0].wrapping_add(i * 4)))).collect();
            println!(
                "  [{frame}] {address:08X} r0 {:08X} r1 {:08X} r2 {:08X} r3 {:08X} lr {:08X} sp {:08X} s0..3 {} [r0] {}",
                r[0], r[1], r[2], r[3], r[14], r[13], s.join(" "), words.join(" ")
            );
        }
        match system.step(Some(deadline)) {
            StepOutcome::Ran => {}
            other => {
                println!("stopped: {other:?}");
                return false;
            }
        }
    }
    *frame += 1;
    true
}

/// runs a frame, printing each function called with value in r0 the first
/// time it is, by functions, analysis/functions.txt's starts.
fn entered_frame(system: &mut System, input: InputState, value: u32, functions: &std::collections::HashSet<u32>, seen: &mut std::collections::HashSet<(u32, u32)>, frame: &mut u64) -> bool {
    system.set_input(input);
    zakuro_core::services::hid::update(system, input);
    let deadline = system.cpu.cycles + zakuro_core::CYCLES_PER_FRAME;
    while system.cpu.cycles < deadline {
        let r = system.cpu.regs;
        if r[0] == value && functions.contains(&r[15]) && seen.insert((r[15], r[14])) {
            println!("  [{frame}] {:08X} lr {:08X} r1 {:08X} r2 {:08X}", r[15], r[14], r[1], r[2]);
        }
        match system.step(Some(deadline)) {
            StepOutcome::Ran => {}
            other => {
                println!("stopped: {other:?}");
                return false;
            }
        }
    }
    *frame += 1;
    true
}

/// runs a frame one instruction at a time, comparing the watched ranges
/// after each and printing the pc of every instruction that changed them.
fn trace_frame(system: &mut System, input: InputState, watches: &[(u32, u32)], frame: &mut u64) -> bool {
    system.set_input(input);
    zakuro_core::services::hid::update(system, input);
    let mut before: Vec<Vec<u8>> = watches.iter().map(|&(a, n)| read(system, a, n)).collect();
    let deadline = system.cpu.cycles + zakuro_core::CYCLES_PER_FRAME;
    let mut seen: HashMap<(u32, u32), u32> = HashMap::new();
    while system.cpu.cycles < deadline {
        let pc = system.cpu.regs[15];
        let lr = system.cpu.regs[14];
        match system.step(Some(deadline)) {
            StepOutcome::Ran => {}
            other => {
                println!("stopped: {other:?}");
                return false;
            }
        }
        for (i, &(address, len)) in watches.iter().enumerate() {
            let now = read(system, address, len);
            if now != before[i] {
                let count = seen.entry((pc, address)).or_insert(0);
                *count += 1;
                if *count <= 3 {
                    let changed: Vec<String> = (0..len as usize / 4)
                        .filter(|&w| now[w * 4..w * 4 + 4] != before[i][w * 4..w * 4 + 4])
                        .map(|w| {
                            let v = u32::from_le_bytes(now[w * 4..w * 4 + 4].try_into().unwrap());
                            format!("+{:X}={v:08X}({})", w * 4, f32::from_bits(v))
                        })
                        .collect();
                    println!("  write pc={pc:08X} lr={lr:08X} {address:08X} {}", changed.join(" "));
                }
                before[i] = now;
            }
        }
    }
    *frame += 1;
    true
}

/// where 0x006D269C goes on after calling 0x00B12218 for the aim delta,
/// which it left at sp+8, with the aim object in r4 and the frame time in s16.
const AFTER_AIM_DELTA: u32 = 0x006D26C4;

/// runs a frame instruction by instruction, adding dx, dy per frame to the
/// local player's aim delta, divided by the frame time since the delta is a
/// speed.
fn mouse_frame(system: &mut System, input: InputState, dx: f32, dy: f32, frame: &mut u64) -> bool {
    system.set_input(input);
    zakuro_core::services::hid::update(system, input);
    let deadline = system.cpu.cycles + zakuro_core::CYCLES_PER_FRAME;
    while system.cpu.cycles < deadline {
        if system.cpu.regs[15] == AFTER_AIM_DELTA {
            let aim = system.cpu.regs[4];
            let remote = read(system, aim + 0xD5, 1)[0] != 0;
            let dt = f32::from_bits(system.cpu.vfp.regs[16]);
            if !remote && dt > 0.0 {
                let out = system.cpu.regs[13] + 8;
                let x = f32::from_bits(read32(system, out)) + dx / dt;
                let y = f32::from_bits(read32(system, out + 4)) + dy / dt;
                system.memory.write_bytes(out, &x.to_bits().to_le_bytes());
                system.memory.write_bytes(out + 4, &y.to_bits().to_le_bytes());
                if *frame % 30 == 0 {
                    println!("  aim {aim:08X} dt {dt} delta ({x}, {y})");
                }
            }
        }
        match system.step(Some(deadline)) {
            StepOutcome::Ran => {}
            other => {
                println!("stopped: {other:?}");
                return false;
            }
        }
    }
    *frame += 1;
    true
}

/// in the top screen's battle UI pass (0x0060D39C), just after it drew the
/// health bar, where the narration is drawn too.
const TOP_UI_AFTER_HUD: u32 = 0x0060D60C;
/// draws a message's line, shadow then text: r0 the message, r1 the 2D context.
const DRAW_MESSAGE_LINE: u32 = 0x009E9A14;
/// the top HUD's score flush, whose context the narration's message is in.
const SCORE_FLUSH_CALLER: u32 = 0x009F2F90;
/// the narration's message in the 2D context.
const NARRATION: u32 = 0x3124;

/// registers kept while the game runs a call the host made in its place.
struct Saved {
    regs: [u32; 16],
    cpsr: zakuro_cpu::psr::Psr,
    vfp: [u32; 32],
    message: u32,
    position: [u8; 8],
}

/// runs a frame instruction by instruction, and in the top screen's UI pass
/// has the game draw the narration's line there too, at x, y.
fn narration_frame(system: &mut System, input: InputState, x: f32, y: f32, frame: &mut u64) -> bool {
    system.set_input(input);
    zakuro_core::services::hid::update(system, input);
    let deadline = system.cpu.cycles + zakuro_core::CYCLES_PER_FRAME;
    let mut context = None;
    let mut saved: Option<Saved> = None;
    while system.cpu.cycles < deadline {
        let pc = system.cpu.regs[15];
        if pc == SPRITE_FLUSH && system.cpu.regs[14] == SCORE_FLUSH_CALLER {
            context = Some(system.cpu.regs[0]);
        }
        if pc == TOP_UI_AFTER_HUD {
            if let Some(back) = saved.take() {
                system.cpu.regs = back.regs;
                system.cpu.cpsr = back.cpsr;
                system.cpu.vfp.regs = back.vfp;
                system.memory.write_bytes(back.message + 0x14, &back.position);
                // run the instruction the call was made in front of
                if system.step(Some(deadline)) != StepOutcome::Ran {
                    return false;
                }
                continue;
            }
            if let Some(context) = context {
                let message = context + NARRATION;
                let head = read(system, message, 8);
                if head[0] != 0 && u32::from_le_bytes(head[4..8].try_into().unwrap()) != 0 {
                    let position: [u8; 8] = read(system, message + 0x14, 8).try_into().unwrap();
                    saved = Some(Saved { regs: system.cpu.regs, cpsr: system.cpu.cpsr, vfp: system.cpu.vfp.regs, message, position });
                    let mut moved = [0u8; 8];
                    moved[..4].copy_from_slice(&x.to_le_bytes());
                    moved[4..].copy_from_slice(&y.to_le_bytes());
                    system.memory.write_bytes(message + 0x14, &moved);
                    system.cpu.regs[0] = message;
                    system.cpu.regs[1] = context;
                    system.cpu.regs[14] = TOP_UI_AFTER_HUD;
                    system.cpu.regs[15] = DRAW_MESSAGE_LINE;
                    system.cpu.cpsr.thumb = false;
                }
            }
        }
        match system.step(Some(deadline)) {
            StepOutcome::Ran => {}
            other => {
                println!("stopped: {other:?}");
                return false;
            }
        }
    }
    *frame += 1;
    true
}

/// the 2D sprite batch flush, entered with the batch in r0.
const SPRITE_FLUSH: u32 = 0x0092F408;
/// sets the position of the quad being added to a batch: r0 the batch,
/// s0, s1 its corner, s2, s3 its size, s4 its depth.
const SPRITE_RECT: u32 = 0x0092EDA8;

fn quads_frame(system: &mut System, input: InputState, frame: &mut u64) -> bool {
    system.set_input(input);
    zakuro_core::services::hid::update(system, input);
    let deadline = system.cpu.cycles + zakuro_core::CYCLES_PER_FRAME;
    while system.cpu.cycles < deadline {
        if system.cpu.regs[15] == 0x006CF458 && *frame % 2 == 0 {
            let info = system.cpu.regs[1];
            let w: Vec<String> = (0..8).map(|i| format!("{:08X}", read32(system, info + i * 4))).collect();
            println!("  [{frame}] aim tick r0 {:08X} frame info {info:08X}: {}", system.cpu.regs[0], w.join(" "));
        }
        if matches!(system.cpu.regs[15], 0x0060D39C | 0x0060D5E8 | 0x0060D60C | 0x009E9A14 | 0x009E2BB4 | 0x00A10900 | 0x00A191DC | 0x00A13304) {
            println!("  [{frame}] at {:08X} lr {:08X}", system.cpu.regs[15], system.cpu.regs[14]);
        }
        if system.cpu.regs[15] == 0x0060DA58 {
            let (r0, r5, r9, r10) = (system.cpu.regs[0], system.cpu.regs[5], system.cpu.regs[9], system.cpu.regs[10]);
            let portraits = read32(system, r10 + 0xD8);
            println!("  [{frame}] bottom bg call r0 {r0:08X} r9 {r9:08X} r10 {r10:08X} r5 {r5:08X} portraits *(r10+D8) {portraits:08X}");
        }
        if system.cpu.regs[15] == 0x00828E34 {
            println!("  [{frame}] pacing r0 {:08X} r1 {:08X} lr {:08X} sp {:08X}", system.cpu.regs[0], system.cpu.regs[1], system.cpu.regs[14], system.cpu.regs[13]);
        }
        if system.cpu.regs[15] == 0x007C4854 {
            println!("  [{frame}] place reticle r0 {:08X} r1 {:08X} lr {:08X}", system.cpu.regs[0], system.cpu.regs[1], system.cpu.regs[14]);
        }
        if system.cpu.regs[15] == SPRITE_RECT {
            let v = |i: usize| f32::from_bits(system.cpu.vfp.regs[i]);
            println!("  [{frame}] rect caller {:08X} batch {:08X} x {:.1} y {:.1} w {:.1} h {:.1}", system.cpu.regs[14], system.cpu.regs[0], v(0), v(1), v(2), v(3));
        }
        if system.cpu.regs[15] == SPRITE_FLUSH {
            let (batch, caller) = (system.cpu.regs[0], system.cpu.regs[14]);
            let end = read32(system, batch + 0x3C);
            let start = read32(system, batch + 0x40);
            let pointer = read32(system, batch + 0x38);
            let count = end.wrapping_sub(start);
            if std::env::var("KIUTRACE_BT").ok().and_then(|v| u32::from_str_radix(&v, 16).ok()) == Some(caller) {
                // return addresses on the stack: words pointing into code just after a bl
                let sp = system.cpu.regs[13];
                let stack = read(system, sp, 0x400);
                let mut chain = Vec::new();
                for (i, c) in stack.chunks(4).enumerate() {
                    let w = u32::from_le_bytes(c.try_into().unwrap());
                    if (0x0010_0000..0x00C9_6000).contains(&w) && w % 2 == 0 {
                        let before = read32(system, w.wrapping_sub(4));
                        if before >> 24 & 0x0F == 0x0B || (before & 0x0FFF_FFF0) == 0x012F_FF30 {
                            chain.push(format!("sp+{:X}:{w:08X}", i * 4));
                        }
                    }
                }
                let raw: Vec<String> = stack.chunks(4).filter(|c| (0x0010_0000..0x00C9_6000).contains(&u32::from_le_bytes((*c).try_into().unwrap()))).map(|c| format!("{:08X}", u32::from_le_bytes(c.try_into().unwrap()))).collect();
                println!("  stack at {sp:08X}: {}", raw.join(" "));
                println!("  [{frame}] backtrace from {caller:08X} r0 {:08X} r4 {:08X} r5 {:08X} r6 {:08X}: {}", system.cpu.regs[0], system.cpu.regs[4], system.cpu.regs[5], system.cpu.regs[6], chain.join(" "));
            }
            if count > 0 && count <= 0x400 {
                let first = pointer - count * 0x60;
                if std::env::var_os("KIUTRACE_QUADDUMP").is_some() && caller == std::env::var("KIUTRACE_QUADDUMP").ok().and_then(|v| u32::from_str_radix(&v, 16).ok()).unwrap_or(0) {
                    let q = read(system, pointer - count * 0x60, count * 0x60);
                    let words: Vec<String> = q.chunks(4).map(|c| format!("{:08X}", u32::from_le_bytes(c.try_into().unwrap()))).collect();
                    println!("  text quad: {}", words.join(" "));
                    let b = read(system, batch, 0x100);
                    let words: Vec<String> = b.chunks(4).map(|c| format!("{:08X}", u32::from_le_bytes(c.try_into().unwrap()))).collect();
                    println!("  batch: {}", words.join(" "));
                }
                let state = read(system, batch + 0x50, 0x44);
                let words: Vec<String> = state.chunks(4).map(|c| {
                    let w = u32::from_le_bytes(c.try_into().unwrap());
                    format!("{}", f32::from_bits(w))
                }).collect();
                println!("  [{frame}] state {}", words.join(" "));
                print!("  [{frame}] caller {caller:08X} batch {batch:08X} {count} quads:");
                for quad in 0..count.min(24) {
                    let bytes = read(system, first + quad * 0x60, 0x60);
                    let f = |o: usize| f32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
                    let xs = [f(0), f(0x18), f(0x30), f(0x48)];
                    let ys = [f(4), f(0x1C), f(0x34), f(0x4C)];
                    let min = |v: [f32; 4]| v.iter().cloned().fold(f32::MAX, f32::min);
                    let max = |v: [f32; 4]| v.iter().cloned().fold(f32::MIN, f32::max);
                    print!(" ({:.0}..{:.0}, {:.0}..{:.0})", min(xs), max(xs), min(ys), max(ys));
                }
                println!();
            }
        }
        match system.step(Some(deadline)) {
            StepOutcome::Ran => {}
            other => {
                println!("stopped: {other:?}");
                return false;
            }
        }
    }
    *frame += 1;
    true
}

fn read(system: &mut System, address: u32, len: u32) -> Vec<u8> {
    let mut out = vec![0u8; len as usize];
    if system.memory.is_readable(address & !0xFFF, 1) {
        system.memory.read_bytes(address, &mut out);
    }
    out
}

fn read32(system: &mut System, address: u32) -> u32 {
    u32::from_le_bytes(read(system, address, 4).try_into().unwrap())
}

/// looks through the heap for objects that start with an aim vtable.
fn scan(system: &mut System) {
    for base in (0x0800_0000u32..0x4000_0000).step_by(0x1000) {
        if !system.memory.is_writable(base, 1) {
            continue;
        }
        let page = read(system, base, 0x1000);
        for offset in (0..0x1000).step_by(4) {
            let word = u32::from_le_bytes(page[offset..offset + 4].try_into().unwrap());
            if let Some((v, name)) = AIM_VTABLES.iter().find(|(v, _)| word.wrapping_sub(*v).wrapping_add(0x20) < 0x40) {
                let object = base + offset as u32;
                print!("  [vtable {word:08X}, {:+}]", word as i64 - *v as i64);
                let x = f32::from_bits(read32(system, object + 0xA4));
                let y = f32::from_bits(read32(system, object + 0xA8));
                println!("  {object:08X} {name} reticle ({x}, {y})");
            }
        }
    }
}

fn write_ppm(path: &Path, width: u32, height: u32, rgba: &[u8]) {
    let mut file = std::fs::File::create(path).expect("create the screenshot");
    write!(file, "P6\n{width} {height}\n255\n").unwrap();
    let rgb: Vec<u8> = rgba.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    file.write_all(&rgb).unwrap();
}

/// runs a frame, and where the menus outside battle's touch listener has
/// no touch (0x00A391E4), first has the game call function with args, as
/// if called from there, then puts the CPU back.
fn call_frame(system: &mut System, input: InputState, function: u32, args: [u32; 3], frame: &mut u64) -> bool {
    const NO_TOUCH: u32 = 0x00A391E4;
    system.set_input(input);
    zakuro_core::services::hid::update(system, input);
    let deadline = system.cpu.cycles + zakuro_core::CYCLES_PER_FRAME;
    let mut saved = None;
    let mut called = false;
    while system.cpu.cycles < deadline {
        if system.cpu.regs[15] == NO_TOUCH {
            if let Some((regs, cpsr, vfp)) = saved.take() {
                println!("  [{frame}] called {function:08X}: r0 {:08X}", system.cpu.regs[0]);
                system.cpu.regs = regs;
                system.cpu.cpsr = cpsr;
                system.cpu.vfp.regs = vfp;
            } else if !called {
                called = true;
                let cpu = &mut system.cpu;
                saved = Some((cpu.regs, cpu.cpsr, cpu.vfp.regs));
                cpu.regs[..3].copy_from_slice(&args);
                cpu.regs[13] = (cpu.regs[13] - 0x40) & !7;
                cpu.regs[14] = NO_TOUCH;
                cpu.regs[15] = function;
                cpu.cpsr.thumb = false;
            }
        }
        match system.step(Some(deadline)) {
            StepOutcome::Ran => {}
            other => {
                println!("stopped: {other:?}");
                return false;
            }
        }
    }
    if saved.is_some() {
        println!("  the call did not come back in the frame");
    }
    *frame += 1;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use zakuro_core::memory::{MemoryRegion, MemoryState, Permission};

    #[test]
    fn host_timing_preserves_execution_and_counts_syscalls() {
        let make = || {
            let mut system = System::new(Config::default());
            let block = system.memory.phys.allocate(MemoryRegion::Application, 0x1000).unwrap();
            system.memory.map(0x0010_0000, block.addr, 0x1000, Permission::RX, MemoryState::Code);
            // mov r0,1; add r0,r0,1; svc GetSystemTick; b back to add
            for (i, word) in [0xE3A0_0001, 0xE280_0001, 0xEF00_0028, 0xEAFF_FFFC].into_iter().enumerate() {
                assert!(system.memory.write32_privileged(0x0010_0000 + i as u32 * 4, word));
            }
            let id = system.kernel.create_thread("main", 0x0010_0000, 0x0010_0F00, 0, 0x30, 0);
            system.map_tls_page(id);
            system.kernel.schedule(&mut system.cpu, 0);
            system
        };
        let mut control = make();
        let mut measured = make();
        measured.host_timings = Some(zakuro_core::HostTimings::default());
        for _ in 0..30 {
            assert_eq!(control.step(None), measured.step(None));
        }
        assert!(control.cpu.snapshot() == measured.cpu.snapshot(), "host timing changed CPU state");
        assert_eq!(control.cpu.cycles, measured.cpu.cycles);
        let timing = measured.host_timings.as_ref().unwrap();
        assert_eq!(timing.interpreted.calls, 30);
        assert_eq!(timing.syscalls[&0x28].calls, 10);
        assert_eq!(timing.compiled.calls, 0);
    }

    #[test]
    fn peek_reads_executable_memory() {
        let mut system = System::new(Config::default());
        let block = system.memory.phys.allocate(MemoryRegion::Application, 0x1000).unwrap();
        system.memory.map(0x0010_0000, block.addr, 0x1000, Permission::RX, MemoryState::Code);
        assert!(system.memory.write32_privileged(0x0010_0000, 0xE3A0_003C));
        assert_eq!(read32(&mut system, 0x0010_0000), 0xE3A0_003C);
        assert_eq!(read32(&mut system, 0x0020_0000), 0);
    }
}

