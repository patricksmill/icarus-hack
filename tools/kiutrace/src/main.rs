//! Runs Kid Icarus: Uprising headless in Zakuro's interpreter from a script
//! of commands, to find where input reaches the game: screenshots to see
//! where it is, scripted buttons and touches, a scan for aim objects by
//! vtable, and a per-instruction trace of what writes a watched range.
//!
//! usage: kiutrace <rom> <script> [out dir]
//!
//! script commands, one per line, # for comments:
//!   run <frames>                  run with the current input
//!   hold <buttons...>             hold buttons (A B X Y L R START SELECT UP DOWN LEFT RIGHT ZL ZR), none releases
//!   touch <x> <y> / untouch       touch the bottom screen at a pixel, or let go
//!   circle <x> <y>                circle pad, -1..1
//!   drag <x0> <y0> <x1> <y1> <frames>   touch and slide across frames, then let go
//!   shot <name>                   write both screens as out/<name>_top.ppm and _bottom.ppm
//!   shotscaled <name>             the same at the hardware renderer's scale, KIUTRACE_SCALE=<n>
//!   save <file> / load <file>     a Zakuro saved state
//!   scan                          list objects whose first word is an aim vtable
//!   peek <addr> <count>           print count words at addr as hex and float
//!   poke <addr> <value>           write a word, a float when it has a point
//!   code <addr> <word>            write a word of code, past the page's permissions
//!   watch <addr> <len>            add a range to the trace
//!   trace <frames>                run instruction by instruction, printing each write to a watched range
//!   aim <dx> <dy> <frames>        run with Zakuro's own mouse aiming captured, moving dx, dy a frame
//!   mirror                        draw the bottom screen's hearts, item box and power icon on top too
//!   portraits <scale>             draw the portraits on the top screen too at scale, with Zakuro's hook
//!   narrtop                       draw the narration on the top screen too, with Zakuro's hook
//!   pausetop                      draw the pause menu on the top screen too, with Zakuro's hook
//!   hud <scale>                   draw the top screen's HUD and reticle at scale, with Zakuro's hook
//!   quads <frames>                print each 2D sprite batch flushed (0x0092F408): caller, and each quad's box
//!   narr <x> <y> <frames>         run, also drawing the narration line on the top screen at x, y
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
    let config = Config { hardware_renderer: scale > 0, resolution: scale.max(1), find_recompiled: false, new3ds, recompiled, ..Config::default() };
    let mut system = loader::load(&args[1], config).unwrap_or_else(|error| {
        eprintln!("could not load {}: {error}", args[1]);
        std::process::exit(1);
    });
    if let Some(library) = &mut system.recompiled {
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
            "run" => {
                for _ in 0..num(1) {
                    if !run_frame(&mut system, input, &mut frame) {
                        return;
                    }
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
            }
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
            "code" => {
                if !system.memory.write32_privileged(num(1), num(2)) {
                    println!("  could not write {:08X}", num(1));
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
            "overclock" => system.cpu.overclock = num(1).max(1),
            "mirror" => system.aim_mouse.get_or_insert_with(Default::default).mirror_bottom = true,
            "portraits" => system.aim_mouse.get_or_insert_with(Default::default).portrait_scale = float(1),
            "narrtop" => system.aim_mouse.get_or_insert_with(Default::default).narration_on_top = true,
            "pausetop" => system.aim_mouse.get_or_insert_with(Default::default).pause_on_top = true,
            "hud" => system.aim_mouse.get_or_insert_with(Default::default).hud_scale = float(1),
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
            other => eprintln!("line {}: unknown command {other}", number + 1),
        }
    }
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
    if system.memory.is_writable(address & !0xFFF, 1) {
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
