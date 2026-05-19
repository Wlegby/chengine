use crate::{functions::remove_border, functions::remove_border_rook};

pub const EMPTY_PSEUDO_ROOK: [u64; 64] = generate_pseudo_rook();
pub const EMPTY_PSEUDO_BISHOP: [u64; 64] = generate_pseudo_bishop();
pub const EMPTY_PSEUDO_KNIGHT: [u64; 64] = generate_pseudo_knight();
pub const EMPTY_PSEUDO_KING: [u64; 64] = generate_pseudo_king();

pub const WHITE_KING_CASTLE: u16 = 0b110_000100;
pub const WHITE_QUEEN_CASTLE: u16 = 0b10_000100;
pub const BLACK_KING_CASTLE: u16 = 0b111110_111100;
pub const BLACK_QUEEN_CASTLE: u16 = 0b111010_111100;

pub const PROMOTION_QUEEN: u16 = 0b1 << 12;
pub const PROMOTION_ROOK: u16 = 0b10 << 12;
pub const PROMOTION_BISHOP: u16 = 0b11 << 12;
pub const PROMOTION_KNIGHT: u16 = 0b100 << 12;

pub const RAY_BETWEEN: [[u64; 64]; 64] = init_ray_between();

pub const ZOBRIST: ZobristKeys = generate_zobrist();

pub struct ZobristKeys {
    pub pieces: [[[u64; 64]; 6]; 2], // [color][piece][square]
    pub side_to_move: u64,
    pub castling: [u64; 16],
    pub en_passant: [u64; 8],
}

const fn generate_zobrist() -> ZobristKeys {
    // A simple Xorshift PRNG to generate random numbers at compile time
    let mut seed: u64 = 0x98f1071585ceeb0a;

    let mut pieces = [[[0; 64]; 6]; 2];
    let mut c = 0;
    while c < 2 {
        let mut p = 0;
        while p < 6 {
            let mut sq = 0;
            while sq < 64 {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                pieces[c][p][sq] = seed;
                sq += 1;
            }
            p += 1;
        }
        c += 1;
    }

    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    let side_to_move = seed;

    let mut castling = [0; 16];
    let mut i = 0;
    while i < 16 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        castling[i] = seed;
        i += 1;
    }

    let mut en_passant = [0; 8];
    let mut i = 0;
    while i < 8 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        en_passant[i] = seed;
        i += 1;
    }

    ZobristKeys {
        pieces,
        side_to_move,
        castling,
        en_passant,
    }
}

const fn init_ray_between() -> [[u64; 64]; 64] {
    let mut table = [[0; 64]; 64];
    let mut sq1 = 0;

    while sq1 < 64 {
        let mut sq2 = 0;
        while sq2 < 64 {
            let r1 = (sq1 / 8) as i8;
            let f1 = (sq1 % 8) as i8;
            let r2 = (sq2 / 8) as i8;
            let f2 = (sq2 % 8) as i8;

            let dr = r2 - r1;
            let df = f2 - f1;

            let abs_dr = if dr < 0 { -dr } else { dr };
            let abs_df = if df < 0 { -df } else { df };

            // Check if the squares share a rank, file, or diagonal
            if dr == 0 || df == 0 || abs_dr == abs_df {
                let step_r = if dr > 0 {
                    1
                } else if dr < 0 {
                    -1
                } else {
                    0
                };
                let step_f = if df > 0 {
                    1
                } else if df < 0 {
                    -1
                } else {
                    0
                };

                let mut r = r1 + step_r;
                let mut f = f1 + step_f;
                let mut mask = 0u64;

                // Step from sq1 towards sq2, flipping the bits in between,
                // stopping strictly before we reach sq2.
                while r != r2 || f != f2 {
                    mask |= 1 << (r * 8 + f);
                    r += step_r;
                    f += step_f;
                }
                table[sq1][sq2] = mask;
            }
            sq2 += 1;
        }
        sq1 += 1;
    }

    table
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
