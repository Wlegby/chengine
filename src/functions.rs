use crate::board::ColorBoards;
use crate::board::Move;
use crate::board::PType;
use crate::board::Piece;
use crate::constants::PROMOTION_BISHOP;
use crate::constants::PROMOTION_KNIGHT;
use crate::constants::PROMOTION_QUEEN;
use crate::constants::PROMOTION_ROOK;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::_pext_u64;

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
        _ => panic!("Expected valid uci"),
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
            'k' => PROMOTION_KNIGHT,
            _ => panic!("Expected valid uci"),
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
        PROMOTION_KNIGHT => "k",
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
    let c8: u64 = c1 << 8 * 7;

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
            11 => a | c8,
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
    pieces: &mut Vec<Piece>,
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
                        pieces.push(Piece {
                            white: c.is_uppercase(),
                            _type: PType::Pawn,
                            position_idx: idx,
                        });
                    }
                    'r' => {
                        boards.rook |= 1 << idx;
                        pieces.push(Piece {
                            white: c.is_uppercase(),
                            _type: PType::Rook,
                            position_idx: idx,
                        });
                    }
                    'n' => {
                        boards.knight |= 1 << idx;
                        pieces.push(Piece {
                            white: c.is_uppercase(),
                            _type: PType::Knight,
                            position_idx: idx,
                        });
                    }
                    'b' => {
                        boards.bishop |= 1 << idx;
                        pieces.push(Piece {
                            white: c.is_uppercase(),
                            _type: PType::Bishop,
                            position_idx: idx,
                        });
                    }
                    'k' => {
                        boards.king |= 1 << idx;
                    }
                    'q' => {
                        boards.queen |= 1 << idx;
                        pieces.push(Piece {
                            white: c.is_uppercase(),
                            _type: PType::Queen,
                            position_idx: idx,
                        });
                    }
                    _ => panic!("Invalid piece type"),
                }

                idx += 1;
            } else {
                idx += c.to_string().parse::<u64>().expect("expected number");
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
