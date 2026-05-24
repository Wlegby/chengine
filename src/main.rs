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
use rayon::prelude::*;
use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use vampirc_uci::{parse_one, UciMessage};

include!(concat!(env!("OUT_DIR"), "/tables.rs"));

fn main() {
    start_uci();
    // debug();
}

fn debug() {
    let mut state = State::from_fen("8/8/1Rr3k1/8/8/8/8/1K6 b - - 0 1");

    // let mut state = State::default();
    state.white_to_move = false;

    let moves = state.get_moves();

    display_bit_board(RAY_BETWEEN.1[42][46]);

    for m in moves {
        display_move(m);
    }
}

fn start_uci() {
    let stdin = io::stdin();
    let mut state = State::default();
    let mut is_self_white = true;

    // Create a shared atomic flag to signal when to stop searching
    let stop_search = Arc::new(AtomicBool::new(false));

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
                } else if startpos {
                    state = State::default();
                }

                state.hash = state.calculate_initial_hash();

                for m in uci_moves {
                    let _move = uci_to_move(&m.to_string());
                    state.make_move(_move);
                }

                is_self_white = state.white_to_move;
            }
            UciMessage::Go { .. } => {
                let mut state_clone = state.clone();
                let stop_search_clone = Arc::clone(&stop_search);

                // Reset the stop flag
                stop_search_clone.store(false, Ordering::Relaxed);

                // Spawn a new thread for the search
                thread::spawn(move || {
                    let mut rng = rand::rng();
                    let moves = state_clone.get_moves();

                    let mut best_move = None;
                    let mut final_eval = 0;

                    // Iterative Deepening: Search from depth 1 to 7
                    for depth in 1..=7 {
                        // Split the first layer of moves across different cores
                        let best_root_result = moves
                            .par_iter()
                            .filter_map(|&m| {
                                // If we've been asked to stop, exit this branch early
                                if stop_search_clone.load(Ordering::Relaxed) {
                                    return None;
                                }

                                // Clone the state and make the root move
                                let mut new_state = state_clone.clone();
                                new_state.make_move(m);

                                // Search the rest of the tree single-threaded from here
                                let (_, opponent_eval) = search(
                                    new_state,
                                    depth - 1,
                                    -i32::MAX, // Starting with open alpha/beta windows
                                    i32::MAX,
                                    &stop_search_clone,
                                );

                                let our_eval = -opponent_eval;

                                // Return the move and its evaluated score
                                Some((m, our_eval))
                            })
                            .max_by_key(|&(_, eval)| eval); // <-- Put them back together to return the best

                        // If we were interrupted by a `stop` command, discard the
                        // incomplete results of this depth and break out.
                        if stop_search_clone.load(Ordering::Relaxed) {
                            break;
                        }

                        // Otherwise, record the completed depth's best move
                        if let Some((m, eval)) = best_root_result {
                            best_move = Some(m);
                            final_eval = eval;

                            // print the info
                            let score_string = format_uci_score(final_eval * 100);
                            let move_str = move_to_uci(m);

                            println!(
                                "info depth {} score {} pv {}",
                                depth, score_string, move_str
                            );
                        }
                    }

                    // Print the best move found before being stopped (or after full depth)
                    if let Some(m) = best_move {
                        println!("bestmove {}", move_to_uci(m));
                    } else if let Some(&fallback_move) = moves.choose(&mut rng) {
                        println!("bestmove {}", move_to_uci(fallback_move));
                    }

                    println!("the score was: {final_eval}");
                });
            }
            UciMessage::Stop => {
                // Tell the search thread to abort
                stop_search.store(true, Ordering::Relaxed);
            }
            UciMessage::Quit => {
                // Ensure we clean up search before exiting
                stop_search.store(true, Ordering::Relaxed);
                return;
            }
            _ => {}
        }
    }
}
