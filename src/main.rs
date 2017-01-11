extern crate clap;

mod cpu;

use cpu::*;
use clap::{Arg,App,SubCommand};

fn main() {
    let matches = App::new("rusty-boy")
        .version("-0.1")
        .author("Mark McCaskey and friends")
        .about("Kappa")
        .arg(Arg::with_name("game")
             .short("g")
             .long("game")
             .value_name("FILE")
             .help("Specifies ROM to load")
             .takes_value(true))
        .get_matches();

    let rom_file = matches.value_of("game").expect("Failed to load ROM");
    let mut gameboy = Cpu::new();

    gameboy.load_rom(rom_file);
    gameboy.play();
}

