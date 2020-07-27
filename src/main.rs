#![allow(non_snake_case)]

extern crate serde;
extern crate serde_json;

extern crate RANKS;

use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::Duration;
use std::{env, fs};

use serde::Serialize;

use RANKS::sim::{Bullet, Configuration, Identity, Tank, Team};
use RANKS::space::Pair;

const WORLD_SIZE: usize = 500;

const DELAY_DURATION: Duration = Duration::from_millis(50);

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
    let config = world.config.clone();
    for (idx, prog) in progs.into_iter().enumerate() {
        let tank = Tank::new(
            Pair::polar((idx as f32) / (progcount as f32) * 2.0 * ::std::f32::consts::PI)
            * 0.75
            * (WORLD_SIZE as f32),
            idx as Team,
            prog,
            config.clone(),
            );
        if let Ok(tank) = tank {
            world.add_tank(tank);
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
