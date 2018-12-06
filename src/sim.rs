use std::cell::RefCell;
use std::rc::Rc;

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
    pub temp: i32,
    pub vm: VM,
    pub dead: bool,
}

impl Tank {
    pub fn apply_heat(&mut self, heat: i32) {
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
                world.bullets.borrow_mut().push(Rc::new(RefCell::new(Bullet{
                    pos: self.pos + Pair::polar(self.aim) * world.config.bullet_s,
                    vel: Pair::polar(self.aim) * world.config.bullet_v,
                    dead: false,
                })));
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

#[derive(Debug,Clone)]
pub struct Bullet {
    pub pos: Pair,
    pub vel: Pair,
    pub dead: bool,
}

impl Entity for Bullet {
    fn step(&mut self, world: &World) {
        self.pos = self.pos + self.vel;
    }
}

#[derive(Debug,Clone)]
pub struct Configuration {
    pub shoot_heat: i32,
    pub idle_heat: i32,
    pub move_heat: i32,
    pub death_heat: i32,
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
            tanks: RefCell::new(Vec::new()),
            bullets: RefCell::new(Vec::new()),
        }
    }
}

#[derive(Debug,Clone)]
pub struct World {
    pub config: Configuration,
    pub tanks: RefCell<Vec<Rc<RefCell<Tank>>>>,
    pub bullets: RefCell<Vec<Rc<RefCell<Bullet>>>>,
}

impl World {
    pub fn add_tank(&mut self, tank: Tank) {
        self.tanks.borrow_mut().push(Rc::new(RefCell::new(tank)));
    }

    pub fn step(&mut self) {
        // All entity steps
        for t in self.tanks.borrow().iter() {
            if !t.borrow().dead {
                t.borrow_mut().step(&self);
            }
        }
        for b in self.bullets.borrow().iter() {
            b.borrow_mut().step(&self);
        }
        
        // All collisions
        enum EntityRef {
            Tank(Rc<RefCell<Tank>>),
            Bullet(Rc<RefCell<Bullet>>),
        }
        let mut root: QuadTreeNode<EntityRef> = QuadTreeBuilder::from_bound(AABB::over_points(
                self.tanks.borrow().iter().map(|t| t.borrow().pos).chain(
                    self.bullets.borrow().iter().map(|b| b.borrow().pos)
                )
        )).build();

        for t in self.tanks.borrow().iter() {
            root.add_pt((t.borrow().pos, EntityRef::Tank(Rc::clone(t))));
        }
        for b in self.bullets.borrow().iter() {
            root.add_pt((b.borrow().pos, EntityRef::Bullet(Rc::clone(b))));
        }

        for t in self.tanks.borrow().iter() {
            let v: Vec<&EntityRef> = root.query(AABB::around(t.borrow().pos, Pair::both(self.config.hit_rad))).map(|(_, r)| r).collect();
            if v.iter().filter(|r| match r {
                &EntityRef::Tank(ref t) => !t.borrow().dead,
                &EntityRef::Bullet(ref b) => !b.borrow().dead,
            }).any(|_| true) {
                for r in v {
                    match r {
                        &EntityRef::Tank(ref t) => t.borrow_mut().dead = true,
                        &EntityRef::Bullet(ref b) => b.borrow_mut().dead = true,
                    }
                }
            }
        }

        // Clean the bullet list, now that we can
        let bullets = self.bullets.borrow().iter().filter(|b| !b.borrow().dead).cloned().collect();
        self.bullets.replace(bullets);
    }

    pub fn finished(&self) -> bool {
        self.tanks.borrow().iter().all(|t| t.borrow().dead)
    }

    pub fn scan(&self, pos: Pair, tm: Team, bounds: (Heading, Heading)) -> (usize, usize) {
        self.tanks.borrow().iter()
            .map(|tank| (tank, (tank.borrow().pos + (-pos)).ang()))
            .filter(|(_t, a)| *a >= (bounds.0).0 && *a < (bounds.1).0)
            .fold((0usize, 0usize), |(us, them), (t, a)| if t.borrow().team == tm { (us + 1, them) } else { (us, them + 1) })
    }

    pub fn explode(&self, pos: Pair, rad: f32) {
        for t in self.tanks.borrow_mut().iter_mut() {
            if (t.borrow().pos + (-pos)).limag() <= rad {
                t.borrow_mut().dead = true;
            }
        }
    }
}
