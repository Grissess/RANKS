extern crate serde;
extern crate serde_json;

extern crate RANKS;

use std::{env, fs};
use std::io::Read;

use RANKS::vm::{Program, Parser, VM, StateBuilder};
use RANKS::sim::{Configuration, Tank, Team};
use RANKS::space::Pair;

const WORLD_SIZE: usize = 500;

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

    for _i in 0..10 {
        world.step();
        println!("json: {}", serde_json::to_string(&*world.tanks.read().unwrap()).unwrap());
        //eprintln!("---\n{:?}", world);
    }
}
