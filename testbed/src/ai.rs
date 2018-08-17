#![allow(dead_code)]

use nalgebra::{
    Point2, Vector2,
    Translation2,
    Rotation2,
    Isometry2,
    UnitComplex,
    distance, distance_squared,
};

use std::time::{Duration, Instant};

pub trait Integrator {
    fn integrate(position: Point2<f32>, velocity: Vector2<f32>) -> Point2<f32>;
}

pub struct Euler;

impl Integrator for Euler {
    fn integrate(position: Point2<f32>, velocity: Vector2<f32>) -> Point2<f32> {
        position + velocity
    }
}

/*
struct Boid {
    max_velocity: f32,
    position: Point2<f32>,
}

fn flee() {
    let desired_velocity = (position - target).normalize() * max_velocity;
    let steering = desired_velocity - velocity;
}

    let steering = truncate (steering, max_force)
    let steering = steering / mass

    let velocity = truncate (velocity + steering , max_speed)
    let position = position + velocity
*/

pub struct Boid {
    position: Point2<f32>,
    velocity: Point2<f32>,

    max_force: f32,
    max_velocity: f32,
    mass: f32,

    max_linear_acceleration: f32,

    path: Vec<Point2<f32>>,
    path_radius: f32,
    current: usize,
}

impl Boid {
    pub fn path_following(&mut self) -> Option<Point2<f32>> {
        if let Some(target) = self.path.get(self.current) {
            if distance_squared(&self.position, target) <= self.path_radius.powi(2) {
                self.current += 1;
                self.current %= self.path.len();
            }
            Some(*target)
            //Some(seek(target))
        } else {
            None
        }
    }
}

pub struct AI {
    pub path: Vec<Point2<f32>>,
    pub path_radius: f32,
    pub current: usize,
    pub v: bool,
    pub last: Instant,
}

impl AI {
    pub fn new() -> Self {
        Self {
            v: false,
            last: Instant::now(),

            path: vec![
                Point2::new(-1.0,  0.0),
                Point2::new( 2.0,  1.0),
                Point2::new(-1.0, -1.0),
            ],
            path_radius: 0.5,
            current: 0,
        }
    }

    pub fn path(&mut self, position: Point2<f32>) -> Option<Vector2<f32>> {
        if let Some(target) = self.path.get(self.current) {
            if distance_squared(&position, target) <= self.path_radius.powi(2) {
                self.current += 1;
                self.current %= self.path.len();
            }
            let steering = seek(&position, target, 2.0);
            Some(steering.translation.vector)
        } else {
            None
        }
    }

    pub fn gen_stick(&mut self, position: Point2<f32>) -> Option<Vector2<f32>> {
        let now = Instant::now();
        let sec = Duration::from_millis(1000);
        if self.last + sec <= now {
            self.last += sec;
            self.v = !self.v;
        }

        self.path(position)
            .map(|v| v.normalize())

        //let x = if self.v { 1.0 } else { -1.0 };
        //let mut stick = Stick::default();
        //stick.x.action(true, self.v);
        //Some(Stick::from_velocity(Vector2::new(x, 0.0)))
        //None
    }
}

pub fn seek(position: &Point2<f32>, target: &Point2<f32>, max_acc: f32) -> Isometry2<f32> {
    let delta = (target - position).normalize();
    Isometry2::from_parts(
        Translation2::from_vector(delta * max_acc),
        UnitComplex::identity(),
    )

    /*
    //let steering = desired - self.velocity;
    let x = desired.x - self.velocity.x;
    let y = desired.y - self.velocity.y;
    Vector2::new(x, y)
    */
}

/*
pub fn flee(actor: &Boid, target: Point2<f32>) -> Isometry2<f32> {
    let delta = (self.position - target).normalize();
    Isometry2::from_parts(
        Translation2::from_vector(delta * actor.max_linear_acceleration),
        UnitComplex::identity(),
    )
}
*/



/*
fn truncate(p: Point2<f32>, max: f32) -> Vector2<f32> {
    let i = max / p.distance(Point2::origin());
    p.scale_by(nalgebra::min(i, 1.0))
}
*/
