extern crate serde;
extern crate serde_json;

extern crate RANKS;

use std::{env, fs};
use std::io::Read;
use std::sync::{Arc,RwLock};
use std::thread::sleep;
use std::time::Duration;

use serde::Serialize;

use RANKS::vm::{Program, Parser, VM, StateBuilder};
use RANKS::sim::{Configuration, Tank, Team, Identity, Bullet};
use RANKS::space::Pair;

const WORLD_SIZE: usize = 500;

const DELAY_DURATION: Duration = Duration::from_millis(100);

#[derive(Serialize)]
struct UpdatePacket<'a>
{
    tanks: &'a Vec<Identity<Arc<RwLock<Tank>>>>,
    bullets: &'a Vec<Identity<Arc<RwLock<Bullet>>>>,
}

fn main() {
    let progs: Vec<Program> = env::args_os().skip(1).map(
            |fname| fs::File::open(fname).expect("Couldn't open file")
        ).map(|mut f| { let mut s = String::new(); f.read_to_string(&mut s).expect("Couldn't read file"); s })
        .map(|s| Program::parse(&s).expect("Parsing failed"))
        .collect();
    let progcount = progs.len();

    eprintln!("{:?}", progs);

    let mut world = Configuration::default().build();
    for (idx, prog) in progs.into_iter().enumerate() {
        world.add_tank(Tank {
            pos: Pair::polar((idx as f32) / (progcount as f32) * 2.0 * ::std::f32::consts::PI) * 0.75 * (WORLD_SIZE as f32),
            aim: 0.0,
            angle: 0.0,
            team: idx as Team,
            temp: 0,
            vm: VM::new(prog, StateBuilder::default().build()),
            dead: false,
        });
    }

    let mut stepnum = 0;
    while (!world.finished())
    {
        world.step();
        println!("Step: {}", stepnum);
        println!("json: {}", serde_json::to_string(&UpdatePacket{
            tanks: &*world.tanks.read().unwrap(),
            bullets: &*world.bullets.read().unwrap(),
        }).unwrap());
        sleep(DELAY_DURATION);
        stepnum += 1;
        //eprintln!("---\n{:?}", world);
    }
}
