//! Prints every function 3dsrecomp finds in a title's executable, one per
//! line as "address mode", for importing into Ghidra.
use recomp3ds::discover::{analyze, Mode};
use recomp3ds::rom::Title;
use recomp3ds::Mods;

fn main() {
    let path = std::env::args().nth(1).expect("usage: funclist <rom>");
    let title = Title::load(&path).expect("could not load the rom");
    for (name, program) in recomp3ds::programs(&title, Mods::default()).expect("could not read the code") {
        if name != "executable" {
            continue;
        }
        for (address, function) in analyze(&program).functions {
            let mode = if matches!(function.mode, Mode::Thumb) { "thumb" } else { "arm" };
            println!("{address:08X} {mode}");
        }
    }
}
