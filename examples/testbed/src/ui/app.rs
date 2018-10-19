use std::{
    rc::Rc,
    sync::Arc,
    time::{Instant, Duration},
    net::SocketAddr,
};
use kiss3d::{
    window::{State, Window},
    text::Font,
    event::{Action, WindowEvent, Key, MouseButton},
    camera::{self, Camera},
    planar_camera::{self, PlanarCamera},
    post_processing::PostProcessingEffect,
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use rayon::prelude::*;
use specs::prelude::*;
use nalgebra::{Point2, Vector2};
use oni::simulator::Simulator;
use crate::{
    client::new_client,
    server::new_server,
    util::*,
    consts::*,
};

use super::{Demo, Text};

pub struct AppState {
    font: Rc<Font>,
    player1: Demo,
    player2: Demo,
    server: Demo,

    camera: camera::FixedView,
    planar_camera: planar_camera::FixedView,

    network: Simulator,

    worker: oni_trace::AppendWorker,

    mouse: Point2<f64>,

    dos: Dos
}

struct Dos {
    server_addr: SocketAddr,
    upd: Instant,
    pool: Arc<ThreadPool>,
    bots: Vec<crate::server::DDOSer>,
    network: Simulator,
}

impl Dos {
    fn new(server_addr: SocketAddr, network: Simulator) -> Self {
        Self {
            server_addr,
            network,
            upd: Instant::now(),
            pool: new_pool("bots", 0, 666),
            bots: Vec::new(),
        }
    }

    fn update(&mut self) {
        if self.upd.elapsed() < Duration::from_millis(33) {
            return;
        }
        self.upd = Instant::now();

        if self.bots.len() < 2 {
            let i = self.bots.len();
            let addr: SocketAddr = format!("[::1]:{}", 3000 + i).parse().unwrap();
            let sock = self.network.add_socket(addr);
            let serv = self.server_addr;
            self.network.add_mapping(serv, addr, SIMULATOR_CONFIG);
            self.network.add_mapping(addr, serv, SIMULATOR_CONFIG);
            self.bots.push(crate::server::DDOSer::new(serv, sock));
        }

        let bots = &mut self.bots;
        self.pool.install(|| {
            bots.par_iter_mut().for_each(|d| {
                oni_trace::scope![update bot];
                d.update()
            });
        });
    }
}

fn new_pool(name: &'static str, num_threads: usize, index: usize) -> Arc<ThreadPool> {
    use oni_trace::register_thread;
    Arc::new(ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .thread_name(move |n| format!("rayon #{} {}", n, name))
        .start_handler(move |_| register_thread(Some(index), Some(index)))
        .build()
        .unwrap())
}

fn new_dispatcher(name: &'static str, num_threads: usize, index: usize) -> DispatcherBuilder<'static, 'static> {
    DispatcherBuilder::new().with_pool(new_pool(name, num_threads, index))
}

impl AppState {
    pub fn new(font: Rc<Font>) -> Self {
        let name = "trace.json.gz";
        let sleep = std::time::Duration::from_millis(100);
        let worker = oni_trace::AppendWorker::new(name, sleep);

        // setup a server, the player's client, and another player.

        let a0 = "[::1]:0000".parse().unwrap();
        let a1 = "[::1]:1111".parse().unwrap();
        let a2 = "[::1]:2222".parse().unwrap();

        let network = Simulator::new();
        let ch0 = network.add_socket_with_name(a0, "server");
        let ch1 = network.add_socket_with_name(a1, "current");
        let ch2 = network.add_socket_with_name(a2, "another");
        network.add_mapping(a0, a1, SIMULATOR_CONFIG);
        network.add_mapping(a0, a2, SIMULATOR_CONFIG);
        network.add_mapping(a1, a0, SIMULATOR_CONFIG);
        network.add_mapping(a2, a0, SIMULATOR_CONFIG);

        let dos = Dos::new(a0, network.clone());

        Self {
            server: new_server(new_dispatcher("server", 1, 2), ch0),
            player1: new_client(new_dispatcher("player1", 1, 3), ch1, a0, false),
            player2: new_client(new_dispatcher("player2", 1, 1), ch2, a0, true),

            mouse: Point2::origin(),
            camera: camera::FixedView::new(),
            planar_camera: planar_camera::FixedView::new(),
            font,
            worker,
            network,

            dos,
        }
    }

    fn events(&mut self, win: &mut Window) {
        for event in win.events().iter() {
            //event.inhibited = true;
            match event.value {
                WindowEvent::Key(Key::Escape, _, _) | WindowEvent::Close => {
                    use std::sync::Once;
                    win.close();

                    static START: Once = Once::new();
                    START.call_once(|| {
                        self.worker.end();
                    });
                }

                WindowEvent::Key(Key::Space, action, _) |
                WindowEvent::MouseButton(MouseButton::Button1, action, _) => {
                    self.player1.client_fire(action == Action::Press);
                    //event.inhibited = true;
                }

                WindowEvent::Key(key, action, _) => {
                    match key {
                        Key::Up | Key::Down | Key::Left | Key::Right =>
                            self.player2.client_arrows(key, action),
                        Key::W | Key::A | Key::S | Key::D =>
                            self.player1.client_wasd(key, action),
                        _ => (),
                    }
                }

                WindowEvent::CursorPos(x, y, _) => {
                    //event.inhibited = true;
                    self.mouse.x = x;
                    self.mouse.y = y;
                }
                _ => (),
            }
        }

        let (x, y) = (self.mouse.x as f32, self.mouse.y as f32);
        self.player1.client_mouse(win, &self.planar_camera, Point2::new(x, y));
    }
}

impl State for AppState {
    fn cameras_and_effect(&mut self) -> (Option<&mut Camera>, Option<&mut PlanarCamera>, Option<&mut PostProcessingEffect>) {
        (Some(&mut self.camera), Some(&mut self.planar_camera), None)
    }

    fn step(&mut self, win: &mut Window) {
        oni_trace::scope![Window Step];

        self.events(win);

        self.dos.update();

        let height = (win.height() as f32) / 3.0;
        self.server.update_view(height * 1.0, height);
        self.player1.update_view(height * 2.0, height);
        self.player2.update_view(height * 0.0, height);

        self.network.advance();

        {
            oni_trace::scope![dispatch];
            self.server.dispatch();
            self.player1.dispatch();
            self.player2.dispatch();
        }

        {
            oni_trace::scope![Run];
            self.server.run(win, &self.planar_camera);
            self.player1.run(win, &self.planar_camera);
            self.player2.run(win, &self.planar_camera);
        }

        let mut text = Text::new(win, self.font.clone());

        //let info = Point2::new(800.0, 10.0);
        //t.info(info, &format!("Lag: {:?}", DEFAULT_LAG));

        self.server.server_status(&mut text, SERVER);
        self.player1.client_status(&mut text, CURRENT, "Current player [WASD+Mouse]");
        self.player2.client_status(&mut text, ANOTHER, "Another player [AI]");

        let size = win.size();
        let size = Vector2::new(size.x as f32, size.y as f32);

        for i in 1..3 {
            let i = i as f32;

            let a = Point2::new(   0.0, height * i as f32);
            let b = Point2::new(size.x, height * i as f32);

            let a = self.planar_camera.unproject(&a, &size);
            let b = self.planar_camera.unproject(&b, &size);

            win.draw_planar_line(&a, &b, &NAVY.into())
        }
    }
}