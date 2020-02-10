// implementation in rust of a chip8 emulator; see
// http://www.multigesture.net/articles/how-to-write-an-emulator-chip-8-interpreter/

#[macro_use]
extern crate clap;
extern crate log;
extern crate sdl2;
extern crate simple_logger;

mod chip8;
use chip8::Chip8;

use std::collections::HashSet;
use std::time::Duration;

use log::{debug, error, info, trace, Level};

use clap::{App, Arg};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

fn main() -> Result<(), String> {
    let matches = App::new("Rust Chip8 emulator")
        .version("1.0")
        .author("Esteban \"truelossless\" Gressard")
        .about("YeT aNoThEr ChIp8 eMuLaToR wRiTtEn In RuSt")
        .arg(
            Arg::with_name("input")
                .help("the .ch8 file to load")
                .required(true)
                .value_name("CH8 FILE")
                .index(1),
        )
        .arg(
            Arg::with_name("verbose")
                .help("how verbose should the emulator be")
                .short("v")
                .long("verbose")
                .value_name("LEVEL")
                .possible_values(&["trace", "debug", "info", "warn", "error"]),
        )
        .arg(
            Arg::with_name("pixel")
            .help("how big should a chip8 pixel be (default: 10)")
            .short("p")
            .long("pixel-size")
            .value_name("SIZE")
        )
        .arg(
            Arg::with_name("speed")
            .help("emulation speed multiplier")
            .short("s")
            .long("speed")
            .value_name("MULTIPLIER")
        )
        
        .get_matches();

    match matches.value_of("verbose").unwrap_or("info") {
        "trace" => simple_logger::init_with_level(Level::Trace).unwrap(),
        "debug" => simple_logger::init_with_level(Level::Debug).unwrap(),
        "warn" => simple_logger::init_with_level(Level::Warn).unwrap(),
        "error" => simple_logger::init_with_level(Level::Error).unwrap(),
        "info" | _ => simple_logger::init_with_level(Level::Info).unwrap(),
    }

    info!("Starting emulator ...");

    // enlargment factor between one chip8 pixel and one real pixel
    // because the chip8 has a really small screen
    let px_size = value_t!(matches, "pixel", u8).unwrap_or(10) as u32;
    trace!("Pixel ratio: {}:1", px_size);

    // speed multiplicator
    let mut speed = value_t!(matches, "speed", u32).unwrap_or(1);
    if speed > 100 {
        speed = 100;
    }

    // emulator initialization
    let mut chip8 = Chip8::new();
    
    let rom_path = matches.value_of("input").unwrap();
    
    if let Err(e) = chip8.load(rom_path) {
        error!("unable to open the file {} !", rom_path);
        error!("full error: {}", e);
        std::process::exit(1);
    } else {
        info!("Loaded file {}", rom_path);
    }

    // sdl2 initialization
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Rust Chip8 emulator", 64 * px_size, 32 * px_size)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    debug!("SDL successfully initialized.");

    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        // get all the pressed keys
        let keys: HashSet<Keycode> = event_pump
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(Keycode::from_scancode)
            .collect();

        // clear all the previous pressed keys
        chip8.clear_keys();

        // send the key to the emulator
        for key in keys {
            match key {
                Keycode::Num0 | Keycode::Kp0 => chip8.register_key(0),
                Keycode::Num1 | Keycode::Kp1 => chip8.register_key(1),
                Keycode::Num2 | Keycode::Kp2 => chip8.register_key(2),
                Keycode::Num3 | Keycode::Kp3 => chip8.register_key(3),
                Keycode::Num4 | Keycode::Kp4 => chip8.register_key(4),
                Keycode::Num5 | Keycode::Kp5 => chip8.register_key(5),
                Keycode::Num6 | Keycode::Kp6 => chip8.register_key(6),
                Keycode::Num7 | Keycode::Kp7 => chip8.register_key(7),
                Keycode::Num8 | Keycode::Kp8 => chip8.register_key(8),
                Keycode::Num9 | Keycode::Kp9 => chip8.register_key(9),
                Keycode::A | Keycode::KpA => chip8.register_key(10),
                Keycode::B | Keycode::KpB => chip8.register_key(11),
                Keycode::C | Keycode::KpC => chip8.register_key(12),
                Keycode::D | Keycode::KpD => chip8.register_key(13),
                Keycode::E | Keycode::KpE => chip8.register_key(14),
                Keycode::F | Keycode::KpF => chip8.register_key(15),
                _ => (),
            }
        }

        // run one step of the emulation
        chip8.emulate().unwrap_or_else(|err| println!("{}", err));
        // clear the screen (not the emulator screen)
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        canvas.set_draw_color(Color::RGB(255, 255, 255));

        // draw again the scene using the display state of the emulator
        for (i, row) in chip8.display().iter().enumerate() {
            for (j, &px) in row.iter().enumerate() {
                if px == 1 {
                    let px_rect = Rect::new(
                        i as i32 * px_size as i32,
                        j as i32 * px_size as i32,
                        px_size,
                        px_size,
                    );

                    canvas.fill_rect(px_rect)?;
                }
            }
        }

        canvas.present();

        // achieve 60 fps, as in the chip8 spec
        std::thread::sleep(Duration::new(0, 1_000_000_000 / (60*speed)));
    }

    Ok(())
}
