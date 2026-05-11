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
