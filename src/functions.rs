use rand::rngs::ThreadRng;
use rand::seq::IndexedRandom;
use rand::seq::IteratorRandom;
use rand::RngExt;

use crate::board::ColorBoards;
use crate::board::Move;
use crate::board::PType;
use crate::board::Piece;
use crate::board::State;
use crate::constants::BLACK_BISHOP;
use crate::constants::BLACK_KING;
use crate::constants::BLACK_KNIGHT;
use crate::constants::BLACK_PAWN;
use crate::constants::BLACK_QUEEN;
use crate::constants::BLACK_ROOK;
use crate::constants::PROMOTION_BISHOP;
use crate::constants::PROMOTION_KNIGHT;
use crate::constants::PROMOTION_QUEEN;
use crate::constants::PROMOTION_ROOK;
use crate::constants::WHITE_BISHOP;
use crate::constants::WHITE_KING;
use crate::constants::WHITE_KNIGHT;
use crate::constants::WHITE_PAWN;
use crate::constants::WHITE_QUEEN;
use crate::constants::WHITE_ROOK;
use crate::tt::TTEntry;
use crate::tt::TT;

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::io::Write;
use std::ops::Index;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::_pext_u64;

#[repr(C, packed)]
struct BookEntry {
    position_hash: u64, // 8 bytes Zobrist hash
    num_moves: u8,
    best_moves: [u16; 20],
}

static BOOK_DATA: &[u8] = include_bytes!("../books/opening_book.bin");

pub fn lookup_move(current_zobrist_hash: u64) -> Option<u16> {
    let entries = unsafe {
        std::slice::from_raw_parts(
            BOOK_DATA.as_ptr() as *const BookEntry,
            BOOK_DATA.len() / std::mem::size_of::<BookEntry>(),
        )
    };

    let mut rng = rand::rng();

    match entries.binary_search_by_key(&current_zobrist_hash, |e| e.position_hash) {
        Ok(index) => {
            let range = entries[index].num_moves;
            if range == 0 {
                return None;
            }
            let mut choice = rng.random_range(0..range);
            let m = entries[index].best_moves[choice as usize];
            if m == 0 {
                return None;
            }
            Some(m)
        }
        Err(_) => None,
    }
}

pub fn king_to_corner_endgame(king_square: usize, other_king_square: usize, endgame: f32) -> i32 {
    let mut eval = 0;

    let (self_x, self_y) = (king_square as i32 % 8, king_square as i32 / 8);
    let (other_x, other_y) = (other_king_square as i32 % 8, other_king_square as i32 / 8);

    let dst_center_x = (3 - other_x).max(other_x - 4);
    let dst_center_y = (3 - other_y).max(other_y - 4);
    let tot_dist = dst_center_x + dst_center_y;
    eval += tot_dist;

    let dist_kings_x = (self_x - other_x).abs();
    let dist_kings_y = (self_y - other_y).abs();
    let tot_dist_kings = dist_kings_x + dist_kings_y;
    eval += 14 - tot_dist_kings;

    (eval as f32 * endgame * 10.) as i32
}

pub fn score_move(state: &State, m: Move) -> i32 {
    let from = (m & 0b111111) as usize;
    let to = ((m & (0b111111 << 6)) >> 6) as usize;
    let prom = m & (0b1111 << 12);

    let mut score = 0;

    // improve castling
    if let Some(k) = state.pieces_list[from] {
        if k._type == PType::King && from.abs_diff(to) == 2 {
            score += 9000;
        }
    }

    // improve queen promotion
    if prom == PROMOTION_QUEEN {
        score += 9000;
    } else if prom != 0 {
        score += 1000;
    }

    let position_buf = match state.pieces_list[from] {
        Some(p) => {
            if p.white {
                match p._type {
                    PType::Pawn => WHITE_PAWN[from],
                    PType::Rook => WHITE_ROOK[from],
                    PType::Bishop => WHITE_BISHOP[from],
                    PType::Queen => WHITE_QUEEN[from],
                    PType::King => WHITE_KING[from],
                    PType::Knight => WHITE_KNIGHT[from],
                }
            } else {
                match p._type {
                    PType::Pawn => BLACK_PAWN[from],
                    PType::Rook => BLACK_ROOK[from],
                    PType::Bishop => BLACK_BISHOP[from],
                    PType::Queen => BLACK_QUEEN[from],
                    PType::King => BLACK_KING[from],
                    PType::Knight => BLACK_KNIGHT[from],
                }
            }
        }
        None => 0,
    };

    score += position_buf;

    // improve capturing good pieces with bad pieces
    if let Some(victim) = state.pieces_list[to] {
        if let Some(attacker) = state.pieces_list[from] {
            // Give a high base score for capturing, then add victim value and subtract attacker value
            score += 10000 + victim.score() - attacker.score();
        }
    }

    score
}

pub fn search(
    mut state: State,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    stop_flag: &AtomicBool,
    tt: &mut TT,
    nodes: &mut u64,
) -> (Option<Move>, i32) {
    *nodes += 1;

    // 1. Check if we have been ordered to stop
    if stop_flag.load(Ordering::Relaxed) {
        return (None, state.evaluate());
    }

    if depth == 0 {
        let static_eval = state.evaluate();
        let mut rng = rand::rng();

        // random score for tiebreaking and hopefully making the engine not deterministic
        let jitter = rng.random_range(-3..=3);
        return (None, static_eval + jitter);
    }

    if state.half_move_clock == 99 {
        return (None, 0);
    }

    if state.is_threefold_repetition() {
        return (None, 0);
    }

    let moves = state.get_moves();
    let mut scored_moves: Vec<(Move, i32)> = moves
        .into_iter()
        .map(|m| (m, score_move(&state, m)))
        .collect();

    if scored_moves.is_empty() {
        let (board, other) = if state.white_to_move {
            (state.white, state.black)
        } else {
            (state.black, state.white)
        };

        let score = if board.king & other.attacks != 0 {
            -(2_000_000_000 + depth as i32)
        } else {
            0
        };
        return (None, score);
    }

    let mut best_move = None;
    let mut max_eval = -i32::MAX;

    for i in 0..scored_moves.len() {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        let mut best_idx = i;
        let mut best_score = scored_moves[i].1;

        for j in (i + 1)..scored_moves.len() {
            if scored_moves[j].1 > best_score {
                best_score = scored_moves[j].1;
                best_idx = j;
            }
        }

        scored_moves.swap(i, best_idx);
        let m = scored_moves[i].0;

        let mut new_state = state.clone();
        new_state.make_move(m);

        let tt_index = tt.get_index(new_state.hash);

        let position_seen = tt.is_empty(tt_index);

        let ((best_next_move, opponent_eval), entry_existed) = if position_seen {
            let entry = tt.get_entry(tt_index);
            if depth < entry.depth {
                ((Some(entry.best_move), entry.score), true)
            } else {
                (
                    search(new_state, depth - 1, -beta, -alpha, stop_flag, tt, nodes),
                    false,
                )
            }
        } else {
            (
                search(new_state, depth - 1, -beta, -alpha, stop_flag, tt, nodes),
                false,
            )
        };

        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        let our_eval = -opponent_eval;

        if !entry_existed && let Some(m) = best_next_move {
            let new_entry = TTEntry {
                hash: new_state.hash,
                depth: depth,
                score: our_eval,
                best_move: m,
            };
        }

        // FIX 2: No more random tie_count
        if our_eval > max_eval {
            max_eval = our_eval;
            best_move = Some(m);
        }

        alpha = alpha.max(our_eval);
        if alpha >= beta {
            break; // Standard pruning
        }
    }

    (best_move, max_eval)
}

pub fn uci_to_move(uci: &str) -> Move {
    let col_from = match uci.chars().nth(0).expect("Expected valid uci") {
        'a' => 0,
        'b' => 1,
        'c' => 2,
        'd' => 3,
        'e' => 4,
        'f' => 5,
        'g' => 6,
        'h' => 7,
        _ => {
            println!("{uci}");
            panic!("Expected valid uci");
        }
    };

    let row_from = uci
        .chars()
        .nth(1)
        .expect("Expected valid uci")
        .to_string()
        .parse::<u16>()
        .expect("Expected valid uci")
        - 1;

    let col_to = match uci.chars().nth(2).expect("Expected valid uci") {
        'a' => 0,
        'b' => 1,
        'c' => 2,
        'd' => 3,
        'e' => 4,
        'f' => 5,
        'g' => 6,
        'h' => 7,
        _ => panic!("Expected valid uci"),
    };

    let row_to = uci
        .chars()
        .nth(3)
        .expect("Expected valid uci")
        .to_string()
        .parse::<u16>()
        .expect("Expected valid uci")
        - 1;

    let promotion = if let Some(c) = uci.chars().nth(4) {
        match c {
            'q' => PROMOTION_QUEEN,
            'r' => PROMOTION_ROOK,
            'b' => PROMOTION_BISHOP,
            'n' => PROMOTION_KNIGHT,
            _ => {
                println!("{uci}");
                panic!("Expected valid uci")
            }
        }
    } else {
        0
    };

    let from = col_from + row_from * 8;
    let to = (col_to + row_to * 8) << 6;

    promotion | to | from
}

pub fn move_to_uci(_move: Move) -> String {
    let (prom, to, from) = (
        _move & (0b1111 << 12),
        (_move & (0b111111 << 6)) >> 6,
        _move & 0b111111,
    );

    fn idx_to_uci(idx: u16) -> String {
        let r = idx / 8;
        let c = idx % 8;

        let col = match c {
            0 => "a",
            1 => "b",
            2 => "c",
            3 => "d",
            4 => "e",
            5 => "f",
            6 => "g",
            7 => "h",
            _ => panic!("Expected the index to be in range"),
        };

        format!("{}{}", col, (r + 1).to_string())
    }

    let promotion = match prom {
        PROMOTION_QUEEN => "q",
        PROMOTION_BISHOP => "b",
        PROMOTION_KNIGHT => "n",
        PROMOTION_ROOK => "r",
        _ => "",
    };

    format!("{}{}{}", idx_to_uci(from), idx_to_uci(to), promotion)
}

pub fn pext(blockers: u64, moves: u64) -> u64 {
    if is_x86_feature_detected!("bmi2") {
        unsafe {
            return _pext_u64(blockers, moves);
            // Result: 1001 (Extracts bits 7, 6, 1, and 0 from value)
        }
    } else {
        let mut hash = 0;

        let mut c = 0;
        let mut i = 0;
        while i < 64 {
            if (moves >> i) & 1 == 1 {
                hash |= ((blockers >> i) & 1) << c;
                c += 1;
            }
            i += 1;
        }

        hash
    }
}

pub const fn remove_border_rook(board: u64, idx: u8) -> u64 {
    let a: u64 = 0x0101010101010101;
    let h: u64 = a << 7;
    let c1: u64 = 0xFF;
    let c8: u64 = c1 << (8 * 7);

    let p: u64 = 1 << idx;
    let mut num = 0;

    if p & c1 != 0 {
        num += 1;
    }
    if p & a != 0 {
        num += 2;
    }
    if p & c8 != 0 {
        num += 4;
    }
    if p & h != 0 {
        num += 7;
    }

    board
        & !match num {
            1 => a | h | c8,
            2 => h | c1 | c8,
            3 => h | c8,
            4 => a | h | c1,
            6 => h | c1,
            7 => a | c1 | c8,
            8 => a | c8,
            11 => a | c1,
            _ => a | h | c1 | c8,
        }
}

pub const fn remove_border(board: u64) -> u64 {
    let a: u64 = 0x0101010101010101;
    let h: u64 = a << 7;
    let c1: u64 = 0xFF;
    let c8: u64 = c1 << 8 * 7;

    let border = a | h | c1 | c8;

    board & !border
}

pub fn fen_pos_notation_to_sq_index(pos: &str) -> u64 {
    if pos.len() != 2 {
        panic!("Invalid position");
    }

    let mut chars = pos.chars();

    let column = chars.next().unwrap();
    let row: u8 = chars.next().unwrap().to_string().parse().unwrap();

    let col = match column {
        'a' => 0,
        'b' => 1,
        'c' => 2,
        'd' => 3,
        'e' => 4,
        'f' => 5,
        'g' => 6,
        'h' => 7,
        _ => panic!("Invalid position"),
    };

    let idx = (col + row * 8) as u64;
    idx
}

pub fn fen_positions_to_bitboards(
    fen: &str,
    pieces: &mut [Option<Piece>; 64],
    castling: &str,
) -> (ColorBoards, ColorBoards) {
    let mut white = ColorBoards::default();
    let mut black = ColorBoards::default();

    let rows: Vec<&str> = fen.split('/').rev().collect();

    let mut idx = 0;
    for row in rows {
        for c in row.chars() {
            if c.is_alphabetic() {
                let boards = if c.is_uppercase() {
                    &mut white
                } else {
                    &mut black
                };

                match c.to_ascii_lowercase() {
                    'p' => {
                        boards.pawn |= 1 << idx;
                        pieces[idx] = Some(Piece {
                            white: c.is_uppercase(),
                            _type: PType::Pawn,
                        });
                    }
                    'r' => {
                        boards.rook |= 1 << idx;
                        pieces[idx] = Some(Piece {
                            white: c.is_uppercase(),
                            _type: PType::Rook,
                        });
                    }
                    'n' => {
                        boards.knight |= 1 << idx;
                        pieces[idx] = Some(Piece {
                            white: c.is_uppercase(),
                            _type: PType::Knight,
                        });
                    }
                    'b' => {
                        boards.bishop |= 1 << idx;
                        pieces[idx] = Some(Piece {
                            white: c.is_uppercase(),
                            _type: PType::Bishop,
                        });
                    }
                    'k' => {
                        boards.king |= 1 << idx;
                    }
                    'q' => {
                        boards.queen |= 1 << idx;
                        pieces[idx] = Some(Piece {
                            white: c.is_uppercase(),
                            _type: PType::Queen,
                        });
                    }
                    _ => panic!("Invalid piece type"),
                }

                idx += 1;
            } else {
                idx += c.to_string().parse::<usize>().expect("expected number");
            }
        }
    }

    for c in castling.chars() {
        let board = if c.is_uppercase() {
            &mut white
        } else {
            &mut black
        };

        if c.to_ascii_lowercase() == 'k' {
            board.castle_k = true
        } else if c.to_ascii_lowercase() == 'q' {
            board.castle_q = true
        }
    }

    white.update_all();
    black.update_all();

    (white, black)
}

pub fn format_uci_score(score: i32, depth: u8) -> String {
    let mate_threshold = 900_000; // Example threshold
                                  //

    dbg!(score);
    if score > mate_threshold {
        // Engine is delivering mate.
        // Note: To get the exact moves until mate, you need to track
        // distance-from-root (ply) in your search function.
        let mate_in = (score - 2_000_000_000 + 1) / 2;
        format!("mate {}", 4 - mate_in.abs())
    } else if score < -mate_threshold {
        // Engine is getting mated.
        let mate_in = (-score - 2_000_000_000 + 1) / 2;
        format!("mate -{}", 4 - mate_in.abs())
    } else {
        // Standard positional centipawn score
        format!("cp {}", score)
    }
}
