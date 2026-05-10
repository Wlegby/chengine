// build.rs
use std::env;
use std::fs;
use std::path::Path;

pub const EMPTY_PSEUDO_ROOK: [u64; 64] = generate_pseudo_rook();
pub const EMPTY_PSEUDO_BISHOP: [u64; 64] = generate_pseudo_bishop();
pub const EMPTY_PSEUDO_KNIGHT: [u64; 64] = generate_pseudo_knight();
pub const EMPTY_PSEUDO_KING: [u64; 64] = generate_pseudo_king();

fn main() {
    // 1. Tell Cargo to rerun this script if build.rs changes
    println!("cargo:rerun-if-changed=build.rs");

    // 2. Get the output directory where Cargo wants us to put generated files
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("tables.rs");

    // 3. Generate the data using your existing functions
    let blocked_rook = generate_blocker_rook();
    let blocked_bishop = generate_blocker_bishop();

    // 4. Format the data as Rust source code
    let mut out = String::new();

    // Write BLOCKED_ROOK
    out.push_str("pub const BLOCKED_ROOK: [[u64; 4096]; 64] = [\n");
    for sq in 0..64 {
        out.push_str("    [\n");
        for blocker in 0..4096 {
            out.push_str(&format!("        {},\n", blocked_rook[sq][blocker]));
        }
        out.push_str("    ],\n");
    }
    out.push_str("];\n\n");

    // Write BLOCKED_BISHOP
    out.push_str("pub const BLOCKED_BISHOP: [[u64; 512]; 64] = [\n");
    for sq in 0..64 {
        out.push_str("    [\n");
        for blocker in 0..512 {
            out.push_str(&format!("        {},\n", blocked_bishop[sq][blocker]));
        }
        out.push_str("    ],\n");
    }
    out.push_str("];\n");

    // 5. Write the formatted string to the file
    fs::write(&dest_path, out).unwrap();
}

pub const fn pdep(hash: u64, moves: u64) -> u64 {
    let mut blockers = 0;

    let mut c = 0;
    let mut i = 0;
    while i < 64 {
        if (moves >> i) & 1 == 1 {
            blockers |= (hash & (1 << c)) << (i - c);
            c += 1;
        }
        i += 1;
    }

    blockers
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

pub const fn generate_blocker_rook() -> [[u64; 4096]; 64] {
    let mut blockers = [[0; 4096]; 64];

    let mut sq: u64 = 0;
    while sq < 64 {
        let mut blocker = 0;
        while blocker < 4096 {
            let board = pdep(
                blocker,
                remove_border_rook(EMPTY_PSEUDO_ROOK[sq as usize], sq as u8),
            );
            blockers[sq as usize][blocker as usize] = generate_blocked_rook(sq, board);

            blocker += 1;
        }
        sq += 1;
    }

    blockers
}

pub const fn generate_blocker_bishop() -> [[u64; 512]; 64] {
    let mut blockers = [[0; 512]; 64];

    let mut sq: u64 = 0;
    while sq < 64 {
        let mut blocker = 0;
        while blocker < 512 {
            let board = pdep(blocker, remove_border(EMPTY_PSEUDO_BISHOP[sq as usize]));
            blockers[sq as usize][blocker as usize] = generate_blocked_bishop(sq, board);

            blocker += 1;
        }
        sq += 1;
    }

    blockers
}

const fn generate_pseudo_king() -> [u64; 64] {
    let mut attacks: [u64; 64] = [0; 64];

    let mut sq = 0;
    while sq < 64 {
        let mut board: u64 = 0;

        let a: u64 = 0x0101010101010101;
        let h: u64 = a << 7;

        let p = 1 << sq as u64;

        board |= (p & !h) << 1;
        board |= (p & !a) >> 1;
        board |= p << 8;
        board |= p >> 8;

        board |= (p & !h) >> 7;
        board |= (p & !a) << 7;
        board |= (p & !a) >> 9;
        board |= (p & !h) << 9;

        attacks[sq] = board;

        sq += 1;
    }

    attacks
}

const fn generate_pseudo_knight() -> [u64; 64] {
    let mut attacks: [u64; 64] = [0; 64];

    let mut sq = 0;
    while sq < 64 {
        let mut board: u64 = 0;

        let a: u64 = 0x0101010101010101;
        let ab: u64 = a | a << 1;
        let h: u64 = a << 7;
        let gh: u64 = h | h >> 1;

        let p = 1 << sq as u64;

        board |= (p & !ab) << 6;
        board |= (p & !gh) << 10;
        board |= (p & !a) << 15;
        board |= (p & !h) << 17;

        board |= (p & !gh) >> 6;
        board |= (p & !ab) >> 10;
        board |= (p & !h) >> 15;
        board |= (p & !a) >> 17;

        attacks[sq] = board;

        sq += 1;
    }

    attacks
}

const fn generate_pseudo_bishop() -> [u64; 64] {
    let mut ray_attacks = [0; 64];

    let mut sq = 0;
    while sq < 64 {
        let maindia: u64 = 0x8040201008040201;
        let diag = 8 * (sq as i32 & 7) - (sq as i32 & 56);
        let nort = -diag & (diag >> 31);
        let sout = diag & (-diag >> 31);
        ray_attacks[sq] = (maindia >> sout) << nort & !(1 << sq);
        sq += 1;
    }

    sq = 0;
    while sq < 64 {
        let maindia: u64 = 0x0102040810204080;
        let diag = 56 - 8 * (sq as i32 & 7) - (sq as i32 & 56);
        let nort = -diag & (diag >> 31);
        let sout = diag & (-diag >> 31);

        ray_attacks[sq] |= (maindia >> sout) << nort & !(1 << sq);
        sq += 1
    }

    ray_attacks
}

pub const fn generate_blocked_bishop(sq: u64, blockers: u64) -> u64 {
    let mut attacks = 0;

    let r = sq / 8;
    let c = sq % 8;

    // NE
    let mut i = r + 1;
    let mut j = c + 1;
    while i < 8 && j < 8 {
        let bit = 1 << (i * 8 + j);
        attacks |= bit;
        if (blockers & bit) == bit {
            break;
        }
        i += 1;
        j += 1;
    }

    // SE
    i = r;
    j = c + 1;
    while i > 0 && j < 8 {
        i -= 1;
        let bit = 1 << (i * 8 + j);
        attacks |= bit;
        if (blockers & bit) == bit {
            break;
        }
        j += 1;
    }

    // SW
    i = r;
    j = c;
    while i > 0 && j > 0 {
        i -= 1;
        j -= 1;

        let bit = 1 << (i * 8 + j);
        attacks |= bit;
        if (blockers & bit) == bit {
            break;
        }
    }

    // NW
    i = r + 1;
    j = c;
    while i < 8 && j > 0 {
        j -= 1;
        let bit = 1 << (i * 8 + j);
        attacks |= bit;
        if (blockers & bit) == bit {
            break;
        }
        i += 1;
    }

    attacks
}

const fn generate_pseudo_rook() -> [u64; 64] {
    let mut ray_attacks = [0; 64];

    let mut sq = 0;
    while sq < 64 {
        ray_attacks[sq] = 0xFF << (sq & 56) & !(1 << sq);
        sq += 1;
    }

    sq = 0;
    while sq < 64 {
        ray_attacks[sq] |= 0x0101010101010101 << (sq & 7) & !(1 << sq);
        sq += 1;
    }

    ray_attacks
}

pub const fn generate_blocked_rook(sq: u64, blockers: u64) -> u64 {
    let mut attacks = 0;

    let r = sq / 8;
    let c = sq % 8;

    // North
    let mut i = r + 1;
    while i < 8 {
        let bit = 1 << (i * 8 + c);
        attacks |= bit;
        if (blockers & bit) == bit {
            break;
        }
        i += 1;
    }

    // South
    i = r;
    while i > 0 {
        i -= 1;
        let bit = 1 << (i * 8 + c);
        attacks |= bit;
        if (blockers & bit) == bit {
            break;
        }
    }

    // East
    i = c + 1;
    while i < 8 {
        let bit = 1 << (r * 8 + i);
        attacks |= bit;
        if (blockers & bit) == bit {
            break;
        }
        i += 1;
    }

    // West
    i = c;
    while i > 0 {
        i -= 1;
        let bit = 1 << (r * 8 + i);
        attacks |= bit;
        if (blockers & bit) != 0 {
            break;
        }
    }

    attacks
}
