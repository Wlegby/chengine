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

use rand::prelude::*;
use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::time::Duration;
use std::time::Instant;
use vampirc_uci::{parse_one, UciMessage};

include!(concat!(env!("OUT_DIR"), "/tables.rs"));

fn main() {
    // let mut state = State::default();
    // state.white_to_move = false;
    //
    // let moves = state.get_moves();
    // for m in moves {
    //     println!("{}", move_to_uci(m));
    // }
    start_uci();
}

fn start_uci() {
    let mut rng = rand::rng();
    let stdin = io::stdin();

    let mut state = State::default();
    state.white_to_move = false;

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let message = parse_one(&line);

        match message {
            UciMessage::Uci => {
                println!("id name Rookery");
                println!("id author Wlegby");
                println!("uciok");
            }
            UciMessage::IsReady => {
                println!("readyok");
            }
            UciMessage::UciNewGame => {
                // Reset the board to the standard starting position
                state = State::default();
                state.white_to_move = false;
            }
            UciMessage::Position {
                startpos,
                fen,
                moves: uci_moves,
            } => {
                // Update your internal board.
                // If the GUI sends a custom FEN, load it. Otherwise, load startpos.
                if let Some(custom_fen) = fen {
                    state = State::from_fen(&custom_fen.to_string());
                    state.white_to_move = false;
                } else if startpos {
                    state = State::default();
                    state.white_to_move = false;
                }

                for m in uci_moves {
                    let _move = uci_to_move(&m.to_string());
                }
            }
            UciMessage::Go { .. } => {
                // 2. Generate moves for the current position
                let moves = state.get_moves();

                println!("bestmove {}", move_to_uci(*moves.choose(&mut rng).unwrap()));
            }
            UciMessage::Quit => break,
            _ => {}
        }
    }
}
