extern crate RANKS;

use std::{env, fs};
use std::io::Read;

use RANKS::vm::{Program, Parser, VM, StateBuilder};
use RANKS::sim::{World, Configuration, Tank, Team};
use RANKS::space::Pair;

const world_size: usize = 500;

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
            pos: Pair::polar((idx as f32) / (progcount as f32) * 2.0 * ::std::f32::consts::PI) * 0.75 * (world_size as f32),
            aim: 0.0,
            angle: 0.0,
            team: idx as Team,
            temp: 0,
            vm: VM::new(prog, StateBuilder::default().build()),
            dead: false,
        });
    }

    for i in 0..10 {
        world.step();
        eprintln!("---\n{:?}", world);
    }
}
