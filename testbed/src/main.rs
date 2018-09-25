#![feature(
    const_fn,
    const_let,
    const_int_ops,
    duration_getters,
    type_ascription,
    decl_macro,
    generators,
    generator_trait,
    ptr_offset_from,
    try_trait,
    int_to_from_bytes,
)]

#[macro_use] extern crate log;
#[macro_use] extern crate specs_derive;
#[macro_use] extern crate shred_derive;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate bitflags;
//#[macro_use] extern crate either;

use kiss3d::{
    light::Light,
    window::Window,
    text::Font,
};

macro_rules! decelerator {
    () => {
        if true {
            std::thread::sleep(std::time::Duration::from_micros(5));
            oni::trace::scope![decelerator];
            std::thread::sleep(std::time::Duration::from_millis(3));
        }
    }
}

mod bit_io;
//mod morton;

mod ai;
mod prot;

mod components;

mod input;
mod client;
mod server;
mod util;

mod ui;

mod consts {
    #![allow(dead_code)]

    use std::time::Duration;

    use crate::util::color;

    pub static SERVER_UPDATE_RATE: f32 = 30.0;
    pub static CLIENT_UPDATE_RATE: f32 = 30.0;

    pub static RENDER_TIME: Duration = Duration::from_millis(100);
        //crate::util::secs_to_duration(1.0 / SERVER_UPDATE_RATE);

    pub const SIMULATOR_CONFIG: oni::simulator::Config = oni::simulator::Config {
        latency: Duration::from_millis(250),
        jitter: Duration::from_millis(0),
        loss: 0.0,
        duplicate: 0.0,
    };

    pub const FONT_SIZE: f32 = ACTOR_RADIUS * 2.0;

    pub const DEFAULT_SPEED: f32 = 2.0;
    pub const ACTOR_RADIUS: f32 = 16.0;

    // from http://clrs.cc/
    pub const NAVY: [f32; 3]    = color(0x001F3F);
    pub const BLUE: [f32; 3]    = color(0x0074D9);
    pub const AQUA: [f32; 3]    = color(0x7FDBFF);
    pub const TEAL: [f32; 3]    = color(0x39CCCC);
    pub const OLIVE: [f32; 3]   = color(0x3D9970);
    pub const GREEN: [f32; 3]   = color(0x2ECC40);
    pub const LIME: [f32; 3]    = color(0x01FF70);
    pub const YELLOW: [f32; 3]  = color(0xFFDC00);
    pub const ORANGE: [f32; 3]  = color(0xFF851B);
    pub const RED: [f32; 3]     = color(0xFF4136);
    pub const MAROON: [f32; 3]  = color(0x85144B);
    pub const FUCHSIA: [f32; 3] = color(0xF012BE);
    pub const PURPLE: [f32; 3]  = color(0xB10DC9);
    pub const BLACK: [f32; 3]   = color(0x111111);
    pub const GRAY: [f32; 3]    = color(0xAAAAAA);
    pub const SILVER: [f32; 3]  = color(0xDDDDDD);
    pub const WHITE: [f32; 3]   = color(0xFFFFFF);

    pub const BG: [f32; 3]      = BLACK;

    pub const ANOTHER: [f32; 3] = LIME;
    pub const CURRENT: [f32; 3] = AQUA;
    pub const SERVER: [f32; 3]  = MAROON;
    //pub const INFO: [f32; 3]    = BLACK;

    pub const LAZER: [f32; 3]   = RED;
    pub const FIRE: [f32; 3]    = YELLOW;
    pub const GUN: [f32; 3]     = TEAL;

    pub const FIRE_RADIUS: f32 = 0.2;
    pub const FIRE_LEN: f32 = 5.0;
}

static FIRA_CODE_REGULAR: &[u8] = include_bytes!("../FiraCode-Regular.ttf");

fn main() {
    use crate::consts::*;

    static LOGGER: oni::trace::Logger = oni::trace::Logger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    println!("trace enabled is {}", oni::trace::ENABLED);

    oni::trace::register_thread(None, None);

    {
        // sample
        let module: &'static str = module_path!();
        let file: &'static str = file!();
        let line: u32 = line!();

        println!("{} {}:{}", module, file, line);
    }

    let font = Font::from_bytes(FIRA_CODE_REGULAR).unwrap();
    let mut win = Window::new("TestBeeed");

    let sim = ui::AppState::new(font);

    //win.set_framerate_limit(None);
    win.set_framerate_limit(Some(60));
    win.set_background_color(BG[0], BG[1], BG[2]);
    win.set_point_size(10.0);
    win.set_light(Light::StickToCamera);

    win.render_loop(sim);
}
