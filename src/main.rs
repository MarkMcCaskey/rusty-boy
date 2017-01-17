extern crate clap;
extern crate sdl2;

mod cpu;

use cpu::*;
use clap::{Arg, App};
use sdl2::*;

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
    let sdl_context = sdl2::init().unwrap();
    let controller_subsystem = sdl_context.game_controller().unwrap();

    let available = match controller_subsystem.num_joysticks() {
        Ok(n) => n,
        Err(e) => panic!("Joystick error: {}", e),
    };


    let mut controller = None;

    for id in 0..available {
        if controller_subsystem.is_game_controller(id) {
            println!("Attempting to open controller {}", id);

            match controller_subsystem.open(id) {
                Ok(c) => {
                    // We managed to find and open a game controller,
                    // exit the loop
                    println!("Success: opened \"{}\"", c.name());
                    controller = Some(c);
                    break;
                }
                Err(e) => println!("failed: {:?}", e),
            }

        } else {
            println!("{} is not a game controller", id);
        }
    }


    let controller = match controller {
        Some(c) => c,
        None => panic!("Couldn't open any controller"),
    };

    print!("wat: {}", controller.mapping());


    for event in sdl_context.event_pump().unwrap().wait_iter() {
        use sdl2::event::Event;

        match event {
            Event::ControllerButtonDown { button, .. } => println!("Button {:?} down", button),
            Event::ControllerButtonUp { button, .. } => println!("Button {:?} up", button),
            Event::Quit { .. } => break,
            _ => (),
        }
    }
    //   gameboy.play();
}
