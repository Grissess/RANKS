#![allow(non_snake_case)]
#![feature(exclusive_range_pattern)]
#![feature(try_blocks)]

extern crate num_derive;
extern crate num_traits;
extern crate serde;
extern crate wasmi;
extern crate websocket;
extern crate native_tls;
extern crate bus;

pub mod sim;
pub mod space;
pub mod vm;
pub mod server;
