extern crate clap;
#[macro_use]extern crate log;
extern crate log4rs;
extern crate sdl2;

mod cpu;

use cpu::*;
use clap::{Arg, App};
use sdl2::*;

use log::LogLevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Logger, Root};

use std::time::Duration;
use sdl2::pixels;
use sdl2::keyboard::Keycode;

fn main() {
    /*Set up logging*/
    let stdout = ConsoleAppender::builder().build();

    let requests = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build("log/requests.log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LogLevelFilter::Trace))
        .unwrap();


    /*Command line arguments*/
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

    /*Attempt to read ROM first*/    
    let rom_file = matches.value_of("game").expect("Could not open specified rom");

    /*Set up gameboy*/
    let mut gameboy = Cpu::new();

    /*Set up SDL; input*/
    let sdl_context = sdl2::init().unwrap();
    let controller_subsystem = sdl_context.game_controller().unwrap();
    controller_subsystem.load_mappings("controllers/sneslayout.txt").unwrap();

    let available = match controller_subsystem.num_joysticks() {
        Ok(n) => n,
        Err(e) => {error!("Joystick error: {}", e); 0},//panic!("Joystick error: {}", e),
    };


    let mut controller = None;
    let mut prev_time = 0;

    for id in 0..available {
        if controller_subsystem.is_game_controller(id) {
            debug!("Attempting to open controller {}", id);

            match controller_subsystem.open(id) {
                Ok(c) => {
                    info!("Success: opened controller \"{}\"", c.name());
                    controller = Some(c);
                    break;
                }
                Err(e) => warn!("failed to open controller: {:?}", e),
            }

        } else {
            debug!("{} is not a game controller", id);
        }
    }


    let controller = match controller {
        Some(c) => c,
        None => panic!("Could not open any controller!"),
    };

    trace!("Controller mapping: {}", controller.mapping());


    
    trace!("loading ROM");
    gameboy.load_rom(rom_file);

    /*Set up graphics and window*/
    trace!("Opening window");
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("", 640, 480)
        .position_centered()
        .build()
        .unwrap();

    let mut renderer = window.renderer()
        .accelerated()
        .build()
        .unwrap();


    /*Set up time*/
    let mut timer = sdl_context.timer().unwrap();
    let mut prev_time = 0;

    let mut cycle_count = 0;

    'main: loop {
        for event in sdl_context.event_pump().unwrap().poll_iter() {
            use sdl2::event::Event;

            match event {
                Event::ControllerAxisMotion { axis, value: val, .. } => {
                    let dead_zone = 10000;
                    if val > dead_zone || val < -dead_zone {
                        debug!("Axis {:?} moved to {}", axis, val);
                        //                   match axis {
                        // controller::Axis::LeftX =>,
                        // controller::Axis::LeftY =>,
                        // _ => (),
                        // }
                        //
                    }
                }
                Event::ControllerButtonDown { button, .. } => {
                    debug!("Button {:?} down", button);
                    match button {
                        controller::Button::A => gameboy.press_a(),
                        controller::Button::B => gameboy.press_b(),
                        controller::Button::Back => gameboy.press_select(),
                        controller::Button::Start => gameboy.press_start(),
                        _ => (),
                    }
                }
                Event::ControllerButtonUp { button, .. } => {
                    debug!("Button {:?} up", button);
                    match button {
                        controller::Button::A => gameboy.unpress_a(),
                        controller::Button::B => gameboy.unpress_b(),
                        controller::Button::Back => gameboy.unpress_select(),
                        controller::Button::Start => gameboy.unpress_start(),
                        _ => (),
                    }
                }
                Event::Quit { .. } => {
                        info!("Program exiting!");
                        break 'main;
                    },
                Event::KeyDown { keycode: Some(keycode), .. } => {
                    if keycode == Keycode::Escape {
                        info!("Program exiting!");
                        break 'main;
                    }
                }
                _ => (),
            }
        }

        cycle_count += gameboy.dispatch_opcode() as u64;

        let ticks = cycle_count - prev_time;

        // 16384hz, call inc_div
        // CPU is at 4.194304MHz (or 1.05MHz) 105000000hz
        // hsync at 9198KHz = 9198000hz
        // vsync at 59.73Hz

        if ticks >= 70224 {
            prev_time = cycle_count;
            renderer.clear();

            renderer.present();

        }

        //       std::thread::sleep(Duration::from_millis(100));
    }
}
