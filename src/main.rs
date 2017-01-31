extern crate clap;
#[macro_use] extern crate log;
extern crate log4rs;
extern crate sdl2;
#[macro_use] extern crate nom;
extern crate ncurses;

mod cpu;
mod disasm;
mod debugger;

use cpu::*;
use debugger::*;

use clap::{Arg, App};
use sdl2::*;

use log::LogLevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Logger, Root};

use std::time::Duration;
use sdl2::keyboard::Keycode;
use sdl2::rect::Point;
use sdl2::audio::{AudioCallback, AudioSpecDesired};

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = match self.phase {
                0.0...0.5 => self.volume,
                _ => -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

fn main() {
    /*Set up logging*/
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{h({l})} {m} {n}")))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LogLevelFilter::Debug))
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
             .required(true)
             .index(1)
             .takes_value(true))
        .arg(Arg::with_name("debug")
             .short("d")
             .multiple(true)
             .long("debug")
             .help("Runs in step-by-step debug mode")
             .takes_value(false))
        .get_matches();

    /*Attempt to read ROM first*/    
    let rom_file = matches.value_of("game").expect("Could not open specified rom");
    let debug_mode = matches.is_present("debug");

    if debug_mode {
        info!("Running in debug mode");
        run_debugger(rom_file);
        return ();
    } else {
        let handle = log4rs::init_config(config).unwrap();
    }

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
    //let mut prev_time = 0;

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
    let window = video_subsystem.window(gameboy.get_game_name().as_str(), 640, 480)
        .position_centered()
        .build()
        .unwrap();

    let mut renderer = window.renderer()
        .accelerated()
        .build()
        .unwrap();

    /* set up audio*/
    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: None
    };

    let device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        // Show obtained AudioSpec
        println!("{:?}", spec);

        // initialize the audio callback
        SquareWave {
            phase_inc: 440.0 / spec.freq as f32,
            phase: 0.0,
            volume: 0.01
        }
    }).unwrap();

    let mut run_next_in_debug = true;
   // let mut debug_in_string = String::new();
    /*Set up time*/
    //let timer = sdl_context.timer().unwrap();
    let mut prev_time = 0;

    let mut cycle_count = 0;
    let mut clock_cycles = 0;

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
                        controller::Button::A => {
                            gameboy.press_a();
                            device.resume();
                        },
                        controller::Button::B => gameboy.press_b(),
                        controller::Button::Back => gameboy.press_select(),
                        controller::Button::Start => gameboy.press_start(),
                        _ => (),
                    }
                }

                Event::ControllerButtonUp { button, .. } => {
                    debug!("Button {:?} up", button);
                    match button {
                        controller::Button::A => {
                            gameboy.unpress_a();
                            device.pause();
                        },
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
                    } else if keycode == Keycode::Space {
                        run_next_in_debug = true;
                    }
                }
                _ => (),
            }
        }

 /*       if debug_mode {
            io::stdin().read_line(debug_in_string).Ok();

        }
        */

        let current_op_time
            = if debug_mode && run_next_in_debug || (!debug_mode) {
                run_next_in_debug = false;
                gameboy.dispatch_opcode() as u64
            } else { std::thread::sleep(Duration::from_millis(10)); 0 };

        cycle_count += current_op_time;
        clock_cycles += current_op_time;
        let timer_khz = gameboy.timer_frequency();
        let time_in_ms_per_cycle = (1000.0 / ((timer_khz as f64) * 1000.0)) as u64;
        clock_cycles += cycle_count;

        let ticks = cycle_count - prev_time;

        let time_in_cpu_cycle_per_cycle = ((time_in_ms_per_cycle as f64)/ (1.0 / (4.19 * 1000.0 * 1000.0))) as u64;

        if clock_cycles >= time_in_cpu_cycle_per_cycle {
            //trace!("Incrementing the timer!");
            gameboy.timer_cycle();
            clock_cycles = 0;
        }

        /*
         * Gameboy screen is 256x256
         * only 160x144 are displayed at a time
         *
         * Background tile map is 32x32 of tiles. Scrollx and scrolly
         * determine how this is actually rendered (it wraps)
         * These numbers index the tile data table
         */

        // 16384hz, call inc_div
        // CPU is at 4.194304MHz (or 1.05MHz) 105000000hz
        // hsync at 9198KHz = 9198000hz
        // vsync at 59.73Hz

        let color1 = sdl2::pixels::Color::RGBA(0,0,0,255);
        let color2 = sdl2::pixels::Color::RGBA(255,0,0,255);
        let color3 = sdl2::pixels::Color::RGBA(0,0,255,255);
        let color4 = sdl2::pixels::Color::RGBA(255,255,255,255);
        let color_lookup = [color1, color2, color3, color4];

        //gameboy.timer_cycle();
        
        renderer.set_scale(12.0,12.0);
        //1ms before drawing in terms of CPU time we must throw a vblank interrupt 
        //TODO: figure out what 70224 is and make it a constant (and/or variable based on whether it's GB, SGB, etc.)
        if ticks + time_in_ms_per_cycle >= 70224 {
            gameboy.set_vblank_interrupt_bit();
        }
        if ticks >= 70224 {
            prev_time = cycle_count;
            renderer.set_draw_color(sdl2::pixels::Color::RGBA(255,0,255,255));
            renderer.clear();

            let background = gameboy.get_background_tiles();

            
            
            /*j
            let mut i = 0;
            let mut j = 0;
            //[[;64];(32*32)]
            for &tile in background.iter() {
                for &pixel in tile.iter() {
                    renderer.set_draw_color(color_lookup[pixel as usize]);
                    let point = Point::new((i % 32),(j % 32));
                    renderer.draw_point(point);
                    j = j + 1;
                }
                i = i + 1;
            }*/

            /*
             * macro location = Grid of 32x32 tiles, 
             * micro location = tiles are each 64 pixels (8x8)
             *
             * macro location % 32 is your macro x location 
             * macro location \ 32 (the quotient) is your macro y location
             * micro location % 8 is your micro x location 
             * micro location \ 8 (the quotient) is your micro y location

             * real x = macro + micro x
             * real y = macro + micro y
             */


            for num_tiles in 0..(32*32) { 
                for num_pixels in 0..64 {
                    renderer.set_draw_color(color_lookup[background[num_tiles as usize][num_pixels as usize] as usize]);
                    let macro_x = num_tiles % 32;
                    let macro_y = num_tiles / 32;
                    let micro_x = num_pixels % 8;
                    let micro_y = num_pixels / 8;

                    let point = Point::new(macro_x + micro_x, macro_y + micro_y);
                    renderer.draw_point(point);
                }
            }
            /*
             *   00111100 1110001 00001000
             *   01111110 1110001 00010100
             *   11111111 1110001 00101010
             */


            renderer.present();
            std::thread::sleep(Duration::from_millis(100));
        }

    }
}
