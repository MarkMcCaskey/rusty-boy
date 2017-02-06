extern crate clap;
#[macro_use] extern crate log;
extern crate log4rs;
extern crate sdl2;
extern crate ncurses;

mod cpu;
mod disasm;
mod debugger;

use cpu::*;
use debugger::*;

use std::num::Wrapping;

use clap::{Arg, App};
use sdl2::*;
use sdl2::surface::Surface;

use log::LogLevelFilter;
use log4rs::append::console::ConsoleAppender;
//use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Root};

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

const SCREEN_WIDTH: u32 = 1600;
const SCREEN_HEIGHT: u32 = 900;

const MEM_DISP_WIDTH: i32 = 384;
const MEM_DISP_HEIGHT: i32 = 0xFFFF/MEM_DISP_WIDTH; // TODO check this?
const X_SCALE: f32 = 4.0;
const Y_SCALE: f32 = X_SCALE;

fn save_screenshot(renderer: &sdl2::render::Renderer,
                   filename: String) {
    let window = renderer.window().unwrap();
    let (w,h) = window.size();
    let format = window.window_pixel_format();
    let mut pixels = renderer.read_pixels(None, format).unwrap();
    let slices = pixels.as_mut_slice();
    let pitch = format.byte_size_of_pixels(w as usize) as u32;
    let masks = format.into_masks().unwrap();
    let surface = sdl2::surface::Surface::from_data_pixelmasks(slices, w, h, pitch, masks).unwrap();
    surface.save_bmp(filename);
}


fn screen_coord_to_mem_addr(x: i32, y: i32) -> Option<cpu::MemAddr> {
    let x_scaled = ((x as f32) / X_SCALE) as i32;
    let y_scaled = ((y as f32) / Y_SCALE) as i32;
    // FIXME this check is not correct
    if x_scaled < MEM_DISP_WIDTH && y_scaled < MEM_DISP_HEIGHT + 1 {
        Some((x_scaled + y_scaled*MEM_DISP_WIDTH) as u16)
    } else {
        None
    }
}


#[allow(unused_variables)]
fn main() {
    /*Set up logging*/
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{h({l})} {m} {n}")))
        .build();


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
        .arg(Arg::with_name("trace")
             .short("t")
             .multiple(true)
             .long("trace")
             .help("Runs with verbose trace")
             .takes_value(false))
        .get_matches();


    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(
            if matches.is_present("trace") {
                LogLevelFilter::Trace
            } else {
                LogLevelFilter::Debug
            }))
        .unwrap();

    
    /*Attempt to read ROM first*/    
    let rom_file = matches.value_of("game").expect("Could not open specified rom");
    let debug_mode = matches.is_present("debug");
    
   
    if debug_mode {
        info!("Running in debug mode");
        run_debugger(rom_file);
    } else {
        let handle = log4rs::init_config(config).unwrap();
    }

    /*Set up gameboy*/
    let mut gameboy = Cpu::new();
    gameboy.event_log_enabled = true;

    /*Set up SDL; input*/
    let sdl_context = sdl2::init().unwrap();
    let controller_subsystem = sdl_context.game_controller().unwrap();
    controller_subsystem.load_mappings("controllers/sneslayout.txt").unwrap();

    let available = match controller_subsystem.num_joysticks() {
        Ok(n) => n,
        Err(e) => {error!("Joystick error: {}", e); 0},//panic!("Joystick error: {}", e),
    };


    //let mut prev_time = 0;

    let mut controller = None;
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


    match controller {
        Some(c) => trace!("Controller mapping: {}", c.mapping()),
        None => trace!("Could not open any controller!"),
    };
    
    trace!("loading ROM");
    gameboy.load_rom(rom_file);

    /*Set up graphics and window*/
    trace!("Opening window");
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window(gameboy.get_game_name().as_str(), SCREEN_WIDTH, SCREEN_HEIGHT)
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

    let mut frame_num = Wrapping(0);

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
                Event::MouseButtonDown {x, y, ..} => {
                    match screen_coord_to_mem_addr(x, y) {
                        Some(pc) => {
                            let pc = pc as usize;
                            let mem = gameboy.mem;
                            let (mnem, size) = disasm::pp_opcode(mem[pc] as u8,
                                                                 mem[pc+1] as u8,
                                                                 mem[pc+1] as u8,
                                                                 pc as u16);
                            println!("${:04X} {:?}", pc, mnem);
                        }
                        _ => (),
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

        match renderer.set_scale(X_SCALE, Y_SCALE) {
            Ok(_)  => (),
            Err(_) => error!("Could not set render scale"),
        }
        //1ms before drawing in terms of CPU time we must throw a vblank interrupt 
        //TODO: figure out what 70224 is and make it a constant (and/or variable based on whether it's GB, SGB, etc.)
        if ticks + time_in_ms_per_cycle >= 70224 {
            gameboy.set_vblank_interrupt_bit();
        }
        if ticks >= 500 {//70224 {
            prev_time = cycle_count;
            renderer.set_draw_color(sdl2::pixels::Color::RGBA(255,0,255,255));
            renderer.clear();

            /*j
            for j in 0..256 {
                gameboy.set_mem((0x8000 + j) as usize, j as i8);
                gameboy.set_mem((0x8000 + (j * 2)) as usize, j as i8);
                gameboy.set_mem((0x8000 + (j * 3)) as usize, j as i8);
                gameboy.set_mem((0x9000 + j) as usize, j as i8);
                gameboy.set_mem((0x9000 + (j * 2)) as usize, j as i8);
                gameboy.set_mem((0x9000 + (j * 3)) as usize, j as i8);

                gameboy.set_mem((0xA000 + j) as usize, j as i8);
                gameboy.set_mem((0xA000 + (j * 2)) as usize, j as i8);
                gameboy.set_mem((0xA000 + (j * 3)) as usize, j as i8);

            }
            */

            
            let mut x = 0;
            let mut y = 0;

            for &p in gameboy.mem.iter() {
                use sdl2::pixels::*;

                // renderer.set_draw_color(Color::RGB(r,g,b));
                // renderer.set_draw_color(Color::RGB(p as u8, p as u8, p as u8));
                renderer.set_draw_color(Color::RGB(0 as u8, 0 as u8, p as u8));
                //debug!("pixel at {}, {} is {}", x, y, p);

                let point = Point::new(x, y);


                renderer.draw_point(point);

                //inc coord
                x = (x + 1) % MEM_DISP_WIDTH;
                if x == 0 {
                    y = (y + 1); // % 256; // does this matter?
                    // gameboy.inc_ly();
                }
            }
            

            
            fn addr_to_point(addr: u16) -> Point {
                let x = (addr as i32) % MEM_DISP_WIDTH;
                let y = (addr as i32) / MEM_DISP_WIDTH;
                Point::new(x as i32, y as i32)
            }
            use sdl2::pixels::*;

            // // draw current PC
            // let pc = gameboy.pc;
            // renderer.set_draw_color(Color::RGB(255, 255, 255));
            // renderer.draw_point(addr_to_point(pc));

            fn clamp_color(v: i16) -> u8 {
                if v < 0 {
                    0
                } else if v > 255 {
                    255
                } else {
                    v as u8
                }
            }
            
            fn mix_color(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> (u8, u8, u8) {
                (clamp_color(r1 as i16 + r2 as i16),
                 clamp_color(g1 as i16 + g2 as i16),
                 clamp_color(b1 as i16 + b2 as i16))
            }

            fn scale_col(scale: u8, color: u8) -> u8 {
                clamp_color((color as f32 * (scale as f32 / 255f32)) as i16)
            }

            // How long stuff stays on screen
            // TODO: Should depend on num of cpu cycles and frame delay
            const FADE_DELAY: u64 = 25531;

            // Event visualization
            // TODO: can be used to do partial "smart" redraw, and speed thing up
            for entry in gameboy.events_deq.iter_mut() {
                let timestamp = entry.timestamp;
                let ref event = entry.event;
                {
                    let time_diff = gameboy.cycles - timestamp;
                    if time_diff < FADE_DELAY {
                        let time_norm = 1.0 - (time_diff as f32)/(FADE_DELAY as f32);
                        let colval = (time_norm * 255.0) as u8;
                        match *event {
                            CpuEvent::Read { from: addr } => {
                                let val = gameboy.mem[addr as usize] as u8;
                                let (r, g, b) = mix_color(0, colval, 0,
                                                          scale_col(colval, val/2), 0, val);
                                renderer.set_draw_color(Color::RGB(r, g, b));
                                renderer.draw_point(addr_to_point(addr));
                            }
                            CpuEvent::Write { to: addr } => {
                                let val = gameboy.mem[addr as usize] as u8;
                                let (r, g, b) = mix_color(colval, 0, 0,
                                                          0, scale_col(colval, val/2), val);
                                renderer.set_draw_color(Color::RGB(r, g, b));
                                renderer.draw_point(addr_to_point(addr));
                            }
                            CpuEvent::Execute(addr) => {
                                let val = gameboy.mem[addr as usize] as u8;
                                let (r, g, b) = mix_color(colval, colval, scale_col(colval, val),
                                                          0, 0, 0);
                                renderer.set_draw_color(Color::RGB(r, g, b));
                                renderer.draw_point(addr_to_point(addr));
                            }
                            _ => (),
                        }
                    }
                }
            }

            while !gameboy.events_deq.is_empty() {
                let timestamp = gameboy.events_deq.front().unwrap().timestamp;
                if (gameboy.cycles - timestamp) >= FADE_DELAY {
                    gameboy.events_deq.pop_front();
                } else {
                    break;
                }
            }

            /*
             *   00111100 1110001 00001000
             *   01111110 1110001 00010100
             *   11111111 1110001 00101010
             */

            // TODO add a way to enable/disable this while running
            let record_screen = false;
            if record_screen {
                save_screenshot(&renderer, format!("screen{:010}.bmp", frame_num.0));
                frame_num += Wrapping(1);
            }
            
            renderer.present();
            
            const FRAME_SLEEP: u64 = 1000/120;
            std::thread::sleep(Duration::from_millis(FRAME_SLEEP));
        }

    }
}

// FIXME Results do not look like HSV, really :)
fn hue(h: u8) -> (f64,f64,f64) {
    let nh = (h as f64) / 256.0;
    let r = ((nh as f64) * 6.0 - 3.0).abs() - 1.0;
    let g = 2.0 - (((nh as f64) * 6.0) - 2.0).abs();
    let b = 2.0 - (((nh as f64) * 6.0) - 4.0).abs();

    (r,g,b)
}

fn hsv_to_rgb(h:u8) -> (u8,u8,u8){
    let (r,g,b) = hue(h);

    let adj = |x| (x * 256.0) as u8;

    (adj(r),adj(g),adj(b))
}

