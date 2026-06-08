#![allow(long_running_const_eval)]
#![allow(unused)]

mod board;
mod constants;
mod functions;
mod qol;
mod tt;

use board::*;
use constants::*;
use functions::*;
use qol::*;

use core::num;
use rand::prelude::*;
use rayon::prelude::*;
use std::env;
use std::env::args;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use vampirc_uci::{parse_one, UciMessage};

use crate::tt::TT;

include!(concat!(env!("OUT_DIR"), "/tables.rs"));

fn main() {
    start_uci();
    // debug();
}

fn debug() {
    let mut state = State::from_fen("r3kb1r/pbn4p/1p3p1p/7R/2Bp4/pN6/2K2PP1/4R3 b kq - 1 23");

    let mut tt = TT::new(128);
    let stop = AtomicBool::new(false);

    let now = Instant::now();
    let find = search(state, 4, -i32::MAX, i32::MAX, &stop, &mut tt);

    let after = Instant::now() - now;

    if let (Some(m), score) = find {
        println!(
            "Move: {}\nScore: {score}\nTook {:0.4}",
            move_to_uci(m),
            after.as_secs_f32()
        );
    }
}

fn start_uci() {
    let stdin = io::stdin();
    let mut state = State::default();
    let tt = Arc::new(Mutex::new(TT::new(1024)));
    let mut is_self_white = true;
    let mut num_moves = 0;
    let mut tree_search = false;

    // Create a shared atomic flag to signal when to stop searching
    let stop_search = Arc::new(AtomicBool::new(false));

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let message = parse_one(&line);

        match message {
            UciMessage::Uci => {
                let mut args = args();
                args.next();
                if let Some(version) = args.next() {
                    println!("id name Chengine-v{}", version);
                    println!("id author Wlegby");
                    println!("uciok");
                } else {
                    println!("id name Chengine-vUNKNOWN");
                    println!("id author Wlegby");
                    println!("uciok");
                }
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
                state.pos_history[state.history_idx] = state.hash;

                for m in uci_moves {
                    let mut move_string = m.to_string();

                    let _move = uci_to_move(&move_string);
                    state.make_move(_move);
                    num_moves += 1;
                }

                is_self_white = state.white_to_move;
            }
            UciMessage::Go { .. } => {
                if num_moves <= 11 && !tree_search {
                    if let Some(m) = lookup_move(state.hash) {
                        println!("info depth 0 score 0 pv {}", move_to_uci(m));
                        println!("bestmove {}", move_to_uci(m));
                    } else {
                        tree_search = true;
                    }
                } else {
                    tree_search = true;
                }

                if tree_search {
                    let mut state_clone = state.clone();
                    let stop_search_clone = Arc::clone(&stop_search);

                    // Reset the stop flag
                    stop_search_clone.store(false, Ordering::Relaxed);

                    let tt_clone = Arc::clone(&tt);

                    let max_depth = if state.endgame > 0.75 { 8 } else { 7 };

                    // Spawn a new thread for the search
                    thread::spawn(move || {
                        let mut rng = rand::rng();
                        let mut moves = state_clone.get_moves();

                        let mut locked_tt = tt_clone.lock().unwrap();

                        println!("going depth {max_depth} (endgame: {})", state.endgame);
                        moves.sort_unstable_by_key(|&m| std::cmp::Reverse(score_move(&state, m)));

                        let mut best_move = None;
                        let mut final_eval = 0;

                        // Iterative Deepening: Search from depth 1 to 7
                        for depth in 1..=max_depth {
                            // Search the rest of the tree single-threaded from here
                            let next_move = search(
                                state,
                                depth,
                                -i32::MAX, // Starting with open alpha/beta windows
                                i32::MAX,
                                &stop_search_clone,
                                &mut locked_tt,
                            );

                            // If we were interrupted by a `stop` command, discard the
                            // incomplete results of this depth and break out.
                            if stop_search_clone.load(Ordering::Relaxed) {
                                break;
                            }

                            // Otherwise, record the completed depth's best move
                            if let (Some(m), eval) = next_move {
                                best_move = Some(m);
                                final_eval = eval;

                                // print the info
                                let score_string = format_uci_score(final_eval, depth);
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
