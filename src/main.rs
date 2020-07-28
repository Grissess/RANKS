#![allow(non_snake_case)]

extern crate serde;
extern crate serde_json;
extern crate websocket;

extern crate RANKS;

use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::Duration;
use std::{env, fs};

use serde::Serialize;

use websocket::OwnedMessage;

use RANKS::sim::{Bullet, Configuration, Identity, Tank, Team};
use RANKS::space::Pair;
use RANKS::server::{TankServer, ClientMessage};

const WORLD_SIZE: usize = 500;

const DELAY_DURATION: Duration = Duration::from_millis(1);

#[derive(Serialize)]
struct UpdatePacket<'a> {
    tanks: &'a Vec<Identity<Arc<RwLock<Tank>>>>,
    bullets: &'a Vec<Identity<Arc<RwLock<Bullet>>>>,
}

enum Mode {
    LocalHeadless,
    WebsocketWatch,
}

fn main() {
    fn print_subcommands() {
        println!("Valid subcommands are:");
        println!("local_headless");
        println!("websocket_watch");
    }
    let mode = match env::args_os().nth(1).map(|s| s.into_string()) {
        Some(Ok(s)) => match &s.as_str() {
            &"local_headless" => Mode::LocalHeadless,
            &"websocket_watch" => Mode::WebsocketWatch,
            _ => {
                print_subcommands();
                return;
            }
        }
        _ => {
            print_subcommands();
            return;
        }
    };
    match mode {
        Mode::LocalHeadless => {
            let progs: Vec<Vec<u8>> = env::args_os()
                .skip(2)
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
        Mode::WebsocketWatch => {
            let progs: Vec<Vec<u8>> = env::args_os()
                .skip(2)
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

            let mut server = TankServer::new(Arc::new(OwnedMessage::Text("{}".into()))).unwrap();
            let rx = server.receiver().unwrap();
            server.init();
            let mut client_count = 0usize;
            let mut stepnum = 0;
            while !world.finished() {
                loop {
                    let rc = if client_count == 0 {
                        Ok(rx.recv().unwrap())
                    } else {
                        rx.try_recv()
                    };
                    match rc {
                        Ok(ClientMessage::Connect(team, addr)) => {
                            println!("Connection from {}, team {}", addr.unwrap(), team);
                            client_count += 1;
                        },
                        Ok(ClientMessage::Disconnect(team)) => {
                            println!("Team {} disconnected", team);
                            client_count -= 1;
                        },
                        Err(_) => break,
                        _ => (),
                    }
                }
                world.step();
                println!("Step: {}", stepnum);
                let bcast = OwnedMessage::Text(serde_json::to_string(&UpdatePacket {
                    tanks: &*world.tanks.read().unwrap(),
                    bullets: &*world.bullets.read().unwrap(),
                })
                .unwrap());
                server.broadcaster().broadcast(bcast);
                sleep(DELAY_DURATION);
                stepnum += 1;
                //eprintln!("---\n{:?}", world);
            }
        }
    }
}
