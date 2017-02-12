//! RustyBoy

extern crate clap;
#[macro_use]
extern crate log;
extern crate log4rs;
extern crate sdl2;
extern crate ncurses;

pub mod assembler;
pub mod cpu;
pub mod debugger;
pub mod disasm;
pub mod io;

use cpu::*;
use debugger::*;
use io::sound::*;
use io::constants::*;
use io::input::*;
use io::graphics::*;
use io::memvis;
use io::vidram;


use std::num::Wrapping;
use clap::{Arg, App};
use sdl2::*;
use sdl2::rect::Rect;
use sdl2::rect::Point;

use log::LogLevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Root};

use sdl2::keyboard::Keycode;

pub const NICER_COLOR: sdl2::pixels::Color = sdl2::pixels::Color::RGBA(139, 41, 2, 255);

#[allow(unused_variables)]
fn main() {
    assert!(SCREEN_WIDTH as f32 >= MEM_DISP_WIDTH as f32 * X_SCALE,
            "Mem vis does not fit in screen");

    // Set up logging
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{h({l})} {m} {n}")))
        .build();


    // Command line arguments
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
        .build(Root::builder().appender("stdout").build(if matches.is_present("trace") {
            LogLevelFilter::Trace
        } else {
            LogLevelFilter::Debug
        }))
        .unwrap();


    // Attempt to read ROM first

    let rom_file = matches.value_of("game").expect("Could not open specified rom");
    let debug_mode = matches.is_present("debug");


    if debug_mode {
        info!("Running in debug mode");
        run_debugger(rom_file);
    } else {
        let handle = log4rs::init_config(config).unwrap();
    }

    // Set up gameboy
    let mut gameboy = Cpu::new();

    let mem_val_display_enabled = true;
    let mut event_log_enabled = true;
    // TODO move all init in one place
    gameboy.event_log_enabled = event_log_enabled;

    // Set up SDL; input
    let sdl_context = sdl2::init().unwrap();

    let mut device = setup_audio(&sdl_context);
    //  device.resume();
    // std::thread::sleep_ms(1000);
    // device.pause();
    setup_controller_subsystem(&sdl_context);

    trace!("loading ROM");
    gameboy.load_rom(rom_file);


    // Set up graphics and window
    trace!("Opening window");
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window(gameboy.get_game_name().as_str(),
                SCREEN_WIDTH,
                SCREEN_HEIGHT)
        .position_centered()
        .build()
        .unwrap();

    let mut renderer = window.renderer()
        .accelerated()
        .build()
        .unwrap();


    // Set up time

    // let timer = sdl_context.timer().unwrap();
    let mut prev_time = 0;

    let mut cycle_count = 0;
    let mut clock_cycles = 0;
    let mut prev_hsync_cycles: u64 = 0;

    // Number of frames saved as screenshots
    let mut frame_num = Wrapping(0);

    let mut tile_data_mode_button = Toggle::new(Rect::new(MEM_DISP_WIDTH,
                                                      MEM_DISP_HEIGHT,
                                                      24,
                                                      12),
                                            vec![TileDataSelect::Mode1,
                                                 TileDataSelect::Mode2]);

    // This does not work as intended because of borrowing
    // let mut buttons = Vec::new();
    // buttons.push(tile_data_mode_button);
    
    device.resume();
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
                        }
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
                        }
                        controller::Button::B => gameboy.unpress_b(),
                        controller::Button::Back => gameboy.unpress_select(),
                        controller::Button::Start => gameboy.unpress_start(),
                        _ => (),
                    }
                }
                Event::Quit { .. } => {
                    info!("Program exiting!");
                    break 'main;
                }
                Event::KeyDown { keycode: Some(keycode), .. } => {
                    match keycode {
                        Keycode::Escape => {
                            info!("Program exiting!");
                            break 'main;
                        }
                        Keycode::F3 => {
                            event_log_enabled = !event_log_enabled;
                            gameboy.event_log_enabled = event_log_enabled;
                        }
                        Keycode::R => {
                            gameboy = Cpu::new();
                            gameboy.load_rom(rom_file);
                            gameboy.event_log_enabled = event_log_enabled;
                        }
                        _ => (),
                    }
                }
                Event::MouseButtonDown { x, y, mouse_btn, .. } => {
                    match mouse_btn {
                        sdl2::mouse::MouseButton::Left => {
                            memvis::memvis_handle_click(&gameboy, x, y);
                            let point = Point::new(x / X_SCALE as i32, y / Y_SCALE as i32);

                            if tile_data_mode_button.rect.contains(point) {
                                tile_data_mode_button.click();
                            }
                        }
                        sdl2::mouse::MouseButton::Right => {
                            // Jump to clicked addr and bring cpu back to life
                            match memvis::screen_coord_to_mem_addr(x, y) {
                                Some(pc) => {
                                    gameboy.pc = pc;
                                    gameboy.state = cpu::constants::CpuState::Normal;
                                }
                                _ => (),
                            }
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
        }

        let current_op_time = if gameboy.state != cpu::constants::CpuState::Crashed {
            gameboy.dispatch_opcode() as u64
        } else {
            10 // FIXME think about what to return here or refactor code around this
        };

        cycle_count += current_op_time;
        clock_cycles += current_op_time;
        let timer_khz = gameboy.timer_frequency();
        let time_in_ms_per_cycle = (1000.0 / ((timer_khz as f64) * 1000.0)) as u64;
        clock_cycles += cycle_count;

        let ticks = cycle_count - prev_time;

        let time_in_cpu_cycle_per_cycle =
            ((time_in_ms_per_cycle as f64) / (1.0 / (4.19 * 1000.0 * 1000.0))) as u64;

        if clock_cycles >= time_in_cpu_cycle_per_cycle {
            //           std::thread::sleep_ms(16);
            // trace!("Incrementing the timer!");
            gameboy.timer_cycle();
            clock_cycles = 0;
        }

        let fake_display_hsync = true;
        if fake_display_hsync {
            // update LY respective to cycles spent execing instruction
            loop {
                if cycle_count < prev_hsync_cycles {
                    break;
                }
                gameboy.inc_ly();
                prev_hsync_cycles += CYCLES_PER_HSYNC;
            }
        }

        // Gameboy screen is 256x256
        // only 160x144 are displayed at a time
        //
        // Background tile map is 32x32 of tiles. Scrollx and scrolly
        // determine how this is actually rendered (it wraps)
        // These numbers index the tile data table
        //

        // 16384hz, call inc_div
        // CPU is at 4.194304MHz (or 1.05MHz) 105000000hz
        // hsync at 9198KHz = 9198000hz
        // vsync at 59.73Hz

        let color1 = sdl2::pixels::Color::RGBA(0, 0, 0, 255);
        let color2 = sdl2::pixels::Color::RGBA(255, 0, 0, 255);
        let color3 = sdl2::pixels::Color::RGBA(0, 0, 255, 255);
        let color4 = sdl2::pixels::Color::RGBA(255, 255, 255, 255);
        let color_lookup = [color1, color2, color3, color4];

        match renderer.set_scale(X_SCALE, Y_SCALE) {
            Ok(_) => (),
            Err(_) => error!("Could not set render scale"),
        }

        // 1ms before drawing in terms of CPU time we must throw a vblank interrupt
        // TODO make this variable based on whether it's GB, SGB, etc.

        if ticks >= CPU_CYCLES_PER_VBLANK {
            prev_time = cycle_count;
            renderer.set_draw_color(NICER_COLOR);
            renderer.clear();


            // // draw current PC
            // let pc = gameboy.pc;
            // renderer.set_draw_color(Color::RGB(255, 255, 255));
            // renderer.draw_point(addr_to_point(pc));

            vidram::draw_tile_patterns(&mut renderer, &gameboy, MEM_DISP_WIDTH + 2);

            // TODO add toggle for this also?
            let tile_map_offset = TILE_MAP_1_START;

            let bg_select = tile_data_mode_button.value().unwrap();
            
            let tile_patterns_offset = match bg_select {
                TileDataSelect::Mode1 => TILE_PATTERN_TABLE_1_ORIGIN,
                TileDataSelect::Mode2 => TILE_PATTERN_TABLE_2_ORIGIN,
            };

            vidram::draw_background_buffer(&mut renderer, &gameboy,
                                           tile_map_offset,
                                           tile_patterns_offset);

            if mem_val_display_enabled {
                memvis::draw_memory_values(&mut renderer, &gameboy);
                
                if gameboy.event_log_enabled {
                    memvis::draw_memory_events(&mut renderer, &mut gameboy);
                }
            }


            tile_data_mode_button.draw(&mut renderer);
            

            //   00111100 1110001 00001000
            //   01111110 1110001 00010100
            //   11111111 1110001 00101010
            //

            // TODO add a way to enable/disable this while running
            let record_screen = false;
            if record_screen {
                save_screenshot(&renderer, format!("screen{:010}.bmp", frame_num.0));
                frame_num += Wrapping(1);
            }

             if gameboy.get_sound1() {
                 device.resume();
             } else {
                 device.pause();
             }
            
            let mut sound_system = device.lock();
            sound_system.wave_duty = gameboy.channel1_wave_pattern_duty();
            sound_system.phase_inc = 1.0 /
                                     (131072.0 / ((2048 - gameboy.channel1_frequency())) as f32);
            sound_system.add = gameboy.channel1_sweep_increase();
            //            131072 / (2048 - gb)


            renderer.present();



            //            device.resume();
            // std::thread::sleep_ms(20);
            // device.pause();


            // Visualizations are slow and this is not the best way to do this anyway
            // std::thread::sleep(Duration::from_millis(FRAME_SLEEP));
        }
       
    }
}

// FIXME Results do not look like HSV, really :)
fn hue(h: u8) -> (f64, f64, f64) {
    let nh = (h as f64) / 256.0;
    let r = ((nh as f64) * 6.0 - 3.0).abs() - 1.0;
    let g = 2.0 - (((nh as f64) * 6.0) - 2.0).abs();
    let b = 2.0 - (((nh as f64) * 6.0) - 4.0).abs();

    (r, g, b)
}

#[allow(dead_code)]
fn hsv_to_rgb(h: u8) -> (u8, u8, u8) {
    let (r, g, b) = hue(h);

    let adj = |x| (x * 256.0) as u8;

    (adj(r), adj(g), adj(b))
}
