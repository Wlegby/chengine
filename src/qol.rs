use crate::board::Move;

pub fn display_bit_board(x: u64) -> u64 {
    let mut text = String::new();

    for j in (0..8).rev() {
        for i in 0..8 {
            text.push_str(&format!("{}", (x >> (8 * j + i)) & 1));
        }
        text.push('\n');
    }

    println!("{text}\n\n");

    x
}

pub fn display_move(x: Move) {
    let from = 0b111111 & x;
    let to = (0b111111000000 & x) >> 6;

    let board = (1 << from) | (1 << to);
    display_bit_board(board);
}
