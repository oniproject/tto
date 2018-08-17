use std::sync::atomic::{AtomicBool, Ordering};
use nalgebra::Vector2;
use kiss3d::event::{Action, Key};

#[derive(Debug, Default)]
pub struct Stick {
    x: InputAxis,
    y: InputAxis,
    rotation: f32,
    updated: AtomicBool,
}

impl Clone for Stick {
    fn clone(&self) -> Self {
        Self {
            x: self.x,
            y: self.y,
            rotation: 0.0,
            updated: self.updated.load(Ordering::Relaxed).into(),
        }
    }
}

impl Stick {
    pub fn velocity(&self) -> Vector2<f32> {
        let x = self.x.0.map(|v| if v { 1.0 } else { -1.0 });
        let y = self.y.0.map(|v| if v { 1.0 } else { -1.0 });

        if x.is_none() && y.is_none() {
            Vector2::zeros()
        } else {
            let (x, y) = (x.unwrap_or(0.0), y.unwrap_or(0.0));
            let vel = Vector2::new(x, y);
            vel.normalize()
        }
    }

    pub fn take_updated(&self) -> Option<Vector2<f32>> {
        let updated = if self.x.0.is_none() && self.y.0.is_none() {
            self.updated.swap(false, Ordering::Relaxed)
        } else {
            self.updated.load(Ordering::Relaxed)
        };
        if updated {
            Some(self.velocity())
        } else {
            None
        }
    }

    pub fn rotate(&mut self, rotation: f32) {
        self.updated.fetch_or(self.rotation != rotation, Ordering::Relaxed);
        self.rotation = rotation;
    }

    pub fn wasd(&mut self, key: Key, action: Action) {
        let last = (self.x, self.y);
        match (key, action) {
            (Key::W, action) => self.y.action(action, true ),
            (Key::S, action) => self.y.action(action, false),
            (Key::A, action) => self.x.action(action, false),
            (Key::D, action) => self.x.action(action, true ),
            (_, _) => (),
        }
        self.updated.fetch_or(last != (self.x, self.y), Ordering::Relaxed);
    }

    pub fn arrows(&mut self, key: Key, action: Action) {
        let last = (self.x, self.y);
        match (key, action) {
            (Key::Up   , action) => self.y.action(action, true ),
            (Key::Down , action) => self.y.action(action, false),
            (Key::Left , action) => self.x.action(action, false),
            (Key::Right, action) => self.x.action(action, true ),
            (_, _) => (),
        }
        self.updated.fetch_or(last != (self.x, self.y), Ordering::Relaxed);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct InputAxis(pub Option<bool>);

impl InputAxis {
    pub fn action(&mut self, action: Action, btn: bool) {
        match (action, self.0, btn) {
            (Action::Press  , _          , _    ) => self.0 = Some(btn),

            (Action::Release, None       , _    ) |
            (Action::Release, Some(true ), true ) |
            (Action::Release, Some(false), false) => self.0 = None,

            (Action::Release, Some(true ), false) |
            (Action::Release, Some(false), true ) => (),
        }
    }
}
