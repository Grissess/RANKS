#![allow(non_snake_case)]

extern crate serde;
extern crate serde_json;

extern crate RANKS;

use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::Duration;
use std::{env, fs};

use serde::Serialize;

use RANKS::sim::{Bullet, Configuration, Identity, Tank, TankState, Team};
use RANKS::space::Pair;
use RANKS::vm::VM;

const WORLD_SIZE: usize = 500;

const DELAY_DURATION: Duration = Duration::from_millis(100);

#[derive(Serialize)]
struct UpdatePacket<'a> {
    tanks: &'a Vec<Identity<Arc<RwLock<Tank>>>>,
    bullets: &'a Vec<Identity<Arc<RwLock<Bullet>>>>,
}

fn main() {
    let progs: Vec<Vec<u8>> = env::args_os()
        .skip(1)
        .map(|fname| fs::read(&fname).expect(&format!("Couldn't read file {:#?}", fname)))
        .collect();
    let progcount = progs.len();

    let mut world = Configuration::default().build();
    for (idx, prog) in progs.into_iter().enumerate() {
        let vm = VM::new(prog);
        match vm {
            Ok(vm) => world.add_tank(Tank {
                pos: Pair::polar((idx as f32) / (progcount as f32) * 2.0 * ::std::f32::consts::PI)
                    * 0.75
                    * (WORLD_SIZE as f32),
                aim: 0.0,
                angle: 0.0,
                team: idx as Team,
                instrs_per_step: 30,
                temp: 0,
                vm,
                state: TankState::Free,
                timers: [0],
            }),
            Err(_) => continue,
        }
    }

    let mut stepnum = 0;
    while !world.finished() {
        world.step();
        println!("Step: {}", stepnum);
        println!(
            "json: {}",
            serde_json::to_string(&UpdatePacket {
                tanks: &*world.tanks.read().unwrap(),
                bullets: &*world.bullets.read().unwrap(),
            })
            .unwrap()
        );
        sleep(DELAY_DURATION);
        stepnum += 1;
        //eprintln!("---\n{:?}", world);
    }
}
