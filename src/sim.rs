use std::sync::RwLock;
use std::sync::Arc;

use std::cell::RefCell;

use serde::{Serialize, Serializer};

use ::vm::*;
use ::space::*;

pub type Team = u8;

pub trait Entity {
    fn step(&mut self, world: &World);
}

#[derive(Debug,Clone)]
pub struct Tank {
    pub pos: Pair,
    pub aim: f32,
    pub angle: f32,
    pub team: Team,
    pub temp: isize,
    pub vm: VM,
    pub dead: bool,
}

// Identity type; needed to make Serdes work properly with Arcs.
#[derive(Debug,Clone)]
pub struct Identity<T> (T);

impl<T> std::ops::Deref for Identity<T>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}

impl<T, U> Serialize for Identity<T>
where T: std::ops::Deref<Target=U>, U: Serialize
{
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        self.0.serialize(s)
    }
}

#[derive(Serialize)]
struct TankSerInfo
{
    pos: Pair,
    angle: f32,
    team: Team,
    dead: bool,
}

impl Serialize for Tank
{
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let info = TankSerInfo
        {
            pos: self.pos,
            angle: self.angle,
            team: self.team,
            dead: self.dead,
        };
        info.serialize(s)
    }
}

// impl Serialize for <Tank>
// {
//     fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
//         where S: Serializer
//     {
//         self.serialize(s);
//     }
// }

impl Tank {
    pub fn apply_heat(&mut self, heat: isize) {
        self.temp = self.temp.saturating_add(heat);
        if self.temp < 0 { self.temp = 0; }
    }
}

impl Entity for Tank {
    fn step(&mut self, world: &World) {
        self.apply_heat(world.config.idle_heat);
        match self.vm.run() {
            UpCall::None | UpCall::Yield => (),
            UpCall::Scan => {
                let bounds = if self.vm.state.regs.a < self.vm.state.regs.b {
                    (self.vm.state.regs.a, self.vm.state.regs.b)
                } else {
                    (self.vm.state.regs.b, self.vm.state.regs.a)
                };
                let bounds = (
                    Heading::from(bounds.0),
                    Heading::from(bounds.1),
                );
                let (a, b) = world.scan(self.pos, self.team, bounds);
                self.vm.state.regs.a = a as isize;
                self.vm.state.regs.b = b as isize;
            },
            UpCall::Fire => {
                self.apply_heat(world.config.shoot_heat);
                world.bullets.write().unwrap().push(Identity(Arc::new(RwLock::new(Bullet{
                    pos: self.pos + Pair::polar(self.aim) * world.config.bullet_s,
                    vel: Pair::polar(self.aim) * world.config.bullet_v,
                    dead: false,
                }))));
            },
            UpCall::Aim(hd) => {
                self.aim = hd.0;
            },
            UpCall::Turn(hd) => {
                self.angle = hd.0;
            },
            UpCall::GPS => {
                self.vm.state.regs.a = (self.pos.x as isize) / 4;
                self.vm.state.regs.b = (self.pos.y as isize) / 4;
            },
            UpCall::Temp => {
                self.vm.state.regs.a = self.temp;
            },
            UpCall::Move => {
                self.pos = self.pos + Pair::polar(self.angle) * world.config.tank_v;
            },
            UpCall::Explode => {
                world.explode(self.pos, world.config.explode_rad);
            },
        }
        if self.temp >= world.config.death_heat {
            world.explode(self.pos, world.config.explode_rad);
        }
    }
}

#[derive(Debug,Clone,Serialize)]
pub struct Bullet {
    pub pos: Pair,
    pub vel: Pair,
    pub dead: bool,
}

impl Entity for Bullet {
    fn step(&mut self, _world: &World) {
        self.pos = self.pos + self.vel;
    }
}

#[derive(Debug,Clone)]
pub struct Configuration {
    pub shoot_heat: isize,
    pub idle_heat: isize,
    pub move_heat: isize,
    pub death_heat: isize,
    pub bullet_v: f32,
    pub bullet_s: f32,
    pub hit_rad: f32,
    pub tank_v: f32,
    pub explode_rad: f32,
}

impl Default for Configuration {
    fn default() -> Configuration {
        Configuration {
            shoot_heat: 26,
            idle_heat: -2,
            move_heat: -2,
            death_heat: 300,
            bullet_v: 5.0,
            bullet_s: 30.0,
            hit_rad: 10.0,
            tank_v: 1.0,
            explode_rad: 50.0,
        }
    }
}

impl Configuration {
    pub fn build(self) -> World {
        World {
            config: self,
            tanks: Arc::new(RwLock::new(Vec::new())),
            bullets: Arc::new(RwLock::new(Vec::new())),
            action_queue: RefCell::new(Vec::new()),
        }
    }
}

#[derive(Debug,Clone)]
pub struct World {
    pub config: Configuration,
    pub tanks: Arc<RwLock<Vec<Identity<Arc<RwLock<Tank>>>>>>,
    pub bullets: Arc<RwLock<Vec<Identity<Arc<RwLock<Bullet>>>>>>,
    action_queue: RefCell<Vec<WorldAction>>
}

#[derive(Clone, Debug)]
enum WorldAction
{
    Explode(Pair, f32),  // Center and radius of explosion
}

impl World {
    pub fn add_tank(&mut self, tank: Tank) {
        self.tanks.write().unwrap().push(Identity(Arc::new(RwLock::new(tank))));
    }

    pub fn step(&mut self) {
        // All entity steps
        for t in self.tanks.read().unwrap().iter() {
            if !t.read().unwrap().dead {
                t.write().unwrap().step(&self);
            }
        }
        for b in self.bullets.read().unwrap().iter() {
            b.write().unwrap().step(&self);
        }
        
        // All collisions
        enum EntityRef {
            Tank(Arc<RwLock<Tank>>),
            Bullet(Arc<RwLock<Bullet>>),
        }
        let mut root: QuadTreeNode<EntityRef> = QuadTreeBuilder::from_bound(AABB::over_points(
                self.tanks.read().unwrap().iter().map(|t| t.read().unwrap().pos).chain(
                    self.bullets.read().unwrap().iter().map(|b| b.read().unwrap().pos)
                )
        )).build();

        for t in self.tanks.read().unwrap().iter() {
            root.add_pt((t.read().unwrap().pos, EntityRef::Tank(Arc::clone(t))));
        }
        for b in self.bullets.read().unwrap().iter() {
            root.add_pt((b.read().unwrap().pos, EntityRef::Bullet(Arc::clone(b))));
        }

        for t in self.tanks.read().unwrap().iter() {
            let v: Vec<&EntityRef> = root.query(AABB::around(t.read().unwrap().pos, Pair::both(self.config.hit_rad))).map(|(_, r)| r).collect();
            if v.iter().filter(|r| match r {
                &EntityRef::Tank(ref t) => !t.read().unwrap().dead,
                &EntityRef::Bullet(ref b) => !b.read().unwrap().dead,
            }).any(|_| true) {
                for r in v {
                    match r {
                        &EntityRef::Tank(ref t) => t.write().unwrap().dead = true,
                        &EntityRef::Bullet(ref b) => b.write().unwrap().dead = true,
                    }
                }
            }
        }

        let mut queue = self.action_queue.borrow_mut();
        while let Some(action) = queue.pop()
        {
            match action
            {
                WorldAction::Explode(pos, rad) => self.do_explode(pos, rad),
            }
        }

        // Clean the bullet list, now that we can
        let bullets = self.bullets.read().unwrap().iter().filter(|b| !b.read().unwrap().dead).cloned().collect();
        *self.bullets.write().unwrap() = bullets;
    }

    pub fn finished(&self) -> bool {
        self.tanks.read().unwrap().iter().all(|t| t.read().unwrap().dead)
    }

    pub fn scan(&self, pos: Pair, tm: Team, bounds: (Heading, Heading)) -> (usize, usize) {
        self.tanks.read().unwrap().iter()
            .map(|tank| (tank, (tank.read().unwrap().pos + (-pos)).ang()))
            .filter(|(_t, a)| *a >= (bounds.0).0 && *a < (bounds.1).0)
            .fold((0usize, 0usize), |(us, them), (t, _a)| if t.read().unwrap().team == tm { (us + 1, them) } else { (us, them + 1) })
    }

    fn do_explode(&self, pos: Pair, rad: f32) {
        for t in self.tanks.write().unwrap().iter_mut() {
            if (t.read().unwrap().pos + (-pos)).limag() <= rad {
                t.write().unwrap().dead = true;
            }
        }
    }

    pub fn explode(&self, pos: Pair, rad: f32)
    {
        self.action_queue.borrow_mut().push(WorldAction::Explode(pos, rad));
    }
}
