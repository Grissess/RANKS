use std::sync::Arc;
use std::sync::RwLock;

use std::cell::RefCell;

use serde::{Serialize, Serializer};

use space::*;
use vm::*;

pub type Team = u8;

pub trait Entity {
    fn step(&mut self, world: &World);
}

#[derive(Debug, Clone)]
pub struct Tank {
    pub pos: Pair,
    pub instrs_per_step: usize,
    pub aim: f32,
    pub angle: f32,
    pub team: Team,
    pub temp: i32,
    pub vm: VM,
    pub state: TankState,
    pub timers: [usize; 1],
}

#[derive(Debug, Clone)]
pub enum TankState {
    Dead,
    Free,
    Pending(Upcall),
}

impl PartialEq for TankState {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TankState::Dead, TankState::Dead) => true,
            (TankState::Free, TankState::Free) => true,
            _ => false,
        }
    }
}

// Identity type; needed to make Serdes work properly with Arcs.
#[derive(Debug, Clone)]
pub struct Identity<T>(T);

impl<T> std::ops::Deref for Identity<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, U> Serialize for Identity<T>
where
    T: std::ops::Deref<Target = U>,
    U: Serialize,
{
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

#[derive(Serialize)]
struct TankSerInfo {
    pos: Pair,
    angle: f32,
    aim: f32,
    temp: i32,
    team: Team,
    dead: bool,
}

impl Serialize for Tank {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let info = TankSerInfo {
            pos: self.pos,
            angle: self.angle,
            aim: self.aim,
            temp: self.temp,
            team: self.team,
            dead: self.state == TankState::Dead,
        };
        info.serialize(s)
    }
}

impl Tank {
    pub fn apply_heat(&mut self, heat: i32) {
        self.temp = self.temp.saturating_add(heat);
        if self.temp < 0 {
            self.temp = 0;
        }
    }
}

impl Entity for Tank {
    fn step(&mut self, world: &World) {
        fn timer(uc: &Upcall, instrs_per_step: usize) -> Option<(usize, usize)> {
            match uc {
                uc if uc.alters_world() => Some((0, instrs_per_step)),
                _ => None,
            }
        }
        self.apply_heat(world.config.idle_heat);
        self.vm.begin_step();
        for timer in &mut self.timers {
            *timer = timer.saturating_sub(self.instrs_per_step);
        }
        loop {
            let uc;
            match &mut self.state {
                TankState::Free => {
                    uc = self.vm.run_until(Some(self.instrs_per_step as isize));
                }
                TankState::Dead => break,
                TankState::Pending(_) => {
                    let mut newstate = TankState::Free;
                    core::mem::swap(&mut self.state, &mut newstate);
                    if let TankState::Pending(upcall) = newstate {
                        uc = upcall;
                    } else {
                        unreachable!();
                    }
                }
            }
            match timer(&uc, self.instrs_per_step) {
                None => (),
                Some((idx, maxtime)) => {
                    if self.timers[idx] >= self.instrs_per_step {
                        self.state = TankState::Pending(uc);
                        break;
                    } else {
                        let counter = isize::max(self.timers[idx] as isize, self.vm.counter());
                        self.vm.set_counter(counter);
                        self.timers[idx] = self.vm.counter() as usize + maxtime;
                    }
                }
            }
            match uc {
                Upcall::Scan(hl, hu, rv) => {
                    let bounds = if hl < hu { (hl, hu) } else { (hu, hl) };
                    let (us, them) = world.scan(self.pos, self.team, bounds);
                    *rv.lock().unwrap() = Some(((us as u64) << 32) | them as u64);
                }
                Upcall::Fire => {
                    self.apply_heat(world.config.shoot_heat);
                    world
                        .bullets
                        .write()
                        .unwrap()
                        .push(Identity(Arc::new(RwLock::new(Bullet {
                            pos: self.pos + Pair::polar(self.aim) * world.config.bullet_s,
                            vel: Pair::polar(self.aim) * world.config.bullet_v,
                            dead: false,
                        }))));
                }
                Upcall::Aim(hd) => {
                    self.aim = hd;
                }
                Upcall::Turn(hd) => {
                    self.angle = hd;
                }
                Upcall::GPSX(rv) => {
                    *rv.lock().unwrap() = Some(self.pos.x);
                }
                Upcall::GPSY(rv) => {
                    *rv.lock().unwrap() = Some(self.pos.y);
                }
                Upcall::Temp(rv) => {
                    *rv.lock().unwrap() = Some(self.temp);
                }
                Upcall::Forward => {
                    self.pos = self.pos + Pair::polar(self.angle) * world.config.tank_v;
                }
                Upcall::PostString(s) => {
                    println!("tank posted string: {}", s);
                }
                Upcall::PostI32(s) => {
                    println!("tank posted i32: {}", s);
                },
                Upcall::PostU32(s) => {
                    println!("tank posted u32: {}", s);
                },
                Upcall::PostI64(s) => {
                    println!("tank posted i64: {}", s);
                },
                Upcall::PostU64(s) => {
                    println!("tank posted u64: {}", s);
                },
                Upcall::PostF32(s) => {
                    println!("tank posted f32: {}", s);
                },
                Upcall::PostF64(s) => {
                    println!("tank posted f64: {}", s);
                },
                Upcall::Explode => {
                    println!("tank commiting suicide!");
                    world.explode(self.pos, world.config.explode_rad);
                    break;
                }
                Upcall::None => break,
            }
        }
        if self.temp >= world.config.death_heat {
            println!("tank too hot!");
            world.explode(self.pos, world.config.explode_rad);
        }
    }
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone)]
pub struct Configuration {
    pub shoot_heat: i32,
    pub idle_heat: i32,
    pub move_heat: i32,
    pub death_heat: i32,
    pub instrs_per_step: usize,
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
            instrs_per_step: 30,
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

#[derive(Debug, Clone)]
pub struct World {
    pub config: Configuration,
    pub tanks: Arc<RwLock<Vec<Identity<Arc<RwLock<Tank>>>>>>,
    pub bullets: Arc<RwLock<Vec<Identity<Arc<RwLock<Bullet>>>>>>,
    action_queue: RefCell<Vec<WorldAction>>,
}

#[derive(Clone, Debug)]
enum WorldAction {
    Explode(Pair, f32), // Center and radius of explosion
}

impl World {
    pub fn add_tank(&mut self, tank: Tank) {
        self.tanks
            .write()
            .unwrap()
            .push(Identity(Arc::new(RwLock::new(tank))));
    }

    pub fn step(&mut self) {
        // All entity steps
        for t in self.tanks.read().unwrap().iter() {
            if t.read().unwrap().state != TankState::Dead {
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
            self.tanks
                .read()
                .unwrap()
                .iter()
                .map(|t| t.read().unwrap().pos)
                .chain(
                    self.bullets
                        .read()
                        .unwrap()
                        .iter()
                        .map(|b| b.read().unwrap().pos),
                ),
        ))
        .build();

        for t in self.tanks.read().unwrap().iter() {
            root.add_pt((t.read().unwrap().pos, EntityRef::Tank(Arc::clone(t))));
        }
        for b in self.bullets.read().unwrap().iter() {
            root.add_pt((b.read().unwrap().pos, EntityRef::Bullet(Arc::clone(b))));
        }

        for t in self.tanks.read().unwrap().iter() {
            let v: Vec<&EntityRef> = root
                .query(AABB::around(
                    t.read().unwrap().pos,
                    Pair::both(self.config.hit_rad),
                ))
                .map(|(_, r)| r)
                .collect();
            if v.iter()
                .filter(|r| match r {
                    &EntityRef::Tank(ref t) => t.read().unwrap().state != TankState::Dead,
                    &EntityRef::Bullet(ref b) => !b.read().unwrap().dead,
                })
                .any(|_| true)
            {
                for r in v {
                    match r {
                        &EntityRef::Tank(ref t) => t.write().unwrap().state = TankState::Dead,
                        &EntityRef::Bullet(ref b) => b.write().unwrap().dead = true,
                    }
                }
            }
        }

        let mut queue = self.action_queue.borrow_mut();
        while let Some(action) = queue.pop() {
            match action {
                WorldAction::Explode(pos, rad) => self.do_explode(pos, rad),
            }
        }

        // Clean the bullet list, now that we can
        let bullets = self
            .bullets
            .read()
            .unwrap()
            .iter()
            .filter(|b| !b.read().unwrap().dead)
            .cloned()
            .collect();
        *self.bullets.write().unwrap() = bullets;
    }

    pub fn finished(&self) -> bool {
        self.tanks
            .read()
            .unwrap()
            .iter()
            .all(|t| t.read().unwrap().state == TankState::Dead)
    }

    pub fn scan(&self, pos: Pair, tm: Team, bounds: (f32, f32)) -> (u32, u32) {
        self.tanks
            .read()
            .unwrap()
            .iter()
            .map(|tank| (tank, (tank.read().unwrap().pos + (-pos)).ang()))
            .filter(|(_t, a)| *a >= bounds.0 && *a < bounds.1)
            .fold((0u32, 0u32), |(us, them), (t, _a)| {
                if t.read().unwrap().team == tm {
                    (us + 1, them)
                } else {
                    (us, them + 1)
                }
            })
    }

    fn do_explode(&self, pos: Pair, rad: f32) {
        for t in self.tanks.write().unwrap().iter_mut() {
            if (t.read().unwrap().pos + (-pos)).limag() <= rad {
                t.write().unwrap().state = TankState::Dead;
            }
        }
    }

    pub fn explode(&self, pos: Pair, rad: f32) {
        self.action_queue
            .borrow_mut()
            .push(WorldAction::Explode(pos, rad));
    }
}
