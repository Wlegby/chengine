use crate::board::ColorBoards;
use crate::board::PType;
use crate::board::Piece;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::_pext_u64;

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

    white.update_all();
    black.update_all();

    (white, black)
}
