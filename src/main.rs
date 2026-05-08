#![allow(long_running_const_eval)]
#![allow(unused)]

mod board;
mod constants;
mod functions;
mod qol;

use board::*;
use constants::*;
use functions::*;
use qol::*;

use std::env;
use std::fs;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

include!(concat!(env!("OUT_DIR"), "/tables.rs"));

fn main() {
    let mut state = State::from_fen("7k/8/8/8/8/2KP3r/8/8 w - - 0 1");

    let (white_moves, black_moves) = state.get_all_moves();

    for _move in white_moves {
        display_move(_move);
    }
}
