use crate::*;

#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub white: bool,
    pub _type: PType,
    pub position_idx: u64,
}

#[derive(Clone, Debug)]
pub struct State {
    pub white: ColorBoards,
    pub black: ColorBoards,
    pub all_pieces: u64,
    pub en_passant: u64,
    pub white_to_move: bool,
    pub half_move_clock: usize,
    pub full_move_clock: usize,
    pub pieces_list: Vec<Piece>,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct ColorBoards {
    pub pawn: u64,
    pub rook: u64,
    pub knight: u64,
    pub bishop: u64,
    pub queen: u64,
    pub king: u64,
    pub all: u64,
}

#[derive(Clone, Copy, Debug)]
pub enum PType {
    Pawn,
    Rook,
    Knight,
    Bishop,
    King,
    Queen,
}

// 0000-000000-000000
//    prom to   from
pub type Move = u16;

pub fn get_moves_from_move_board(mut move_board: u64, idx: u64) -> Vec<Move> {
    let mut moves = Vec::new();

    let mut indices = Vec::new();

    while move_board != 0 {
        // trailing_zeros() returns the index of the lowest set bit
        let trail = move_board.trailing_zeros();
        indices.push(trail);

        // Clear the lowest set bit (Brian Kernighan's algorithm)
        move_board &= move_board - 1;
    }

    for index in indices {
        let from = idx as u16;
        let to = (index as u16) << 6;
        moves.push(from | to);
    }

    moves
}

impl PType {
    pub fn sliding(&self) -> bool {
        match self {
            Self::Rook => true,
            Self::Bishop => true,
            Self::Queen => true,
            _ => false,
        }
    }
}

impl ColorBoards {
    pub fn update_all(&mut self) {
        self.all = self.pawn | self.rook | self.knight | self.bishop | self.queen | self.king;
    }
}

impl Default for State {
    fn default() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }
}

impl State {
    pub fn from_fen(fen: &str) -> Self {
        let mut pieces = Vec::new();

        let parts: Vec<&str> = fen.split_whitespace().collect();
        let (pos, to_move, cast, en_pass, half_move, full_move) =
            (parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]);

        let en_passant = if en_pass != "-" {
            1 << fen_pos_notation_to_sq_index(en_pass)
        } else {
            0
        };

        let half_move_clock: usize = half_move
            .parse()
            .expect("Expected a half move clock number");
        let full_move_clock: usize = full_move
            .parse()
            .expect("Expected a full move clock number");

        let (white, black) = fen_positions_to_bitboards(pos, &mut pieces);
        let all_pieces = white.all | black.all;

        let white_to_move = to_move == "w";

        Self {
            white,
            black,
            en_passant,
            white_to_move,
            all_pieces,
            half_move_clock,
            full_move_clock,
            pieces_list: pieces,
        }
    }

    pub fn king_moves(&self, pos_idx: u64, white: bool) -> u64 {
        let board = if white { self.white } else { self.black };

        EMPTY_PSEUDO_KING[pos_idx as usize] & !board.all
    }

    pub fn get_pins(&self, white: bool) -> Vec<usize> {
        let king_idx = if white {
            self.white.king.trailing_zeros() as usize
        } else {
            self.black.king.trailing_zeros() as usize
        };

        let king_straight = EMPTY_PSEUDO_ROOK[king_idx];
        let king_diagonal = EMPTY_PSEUDO_BISHOP[king_idx];
        let king_all = king_straight | king_diagonal;

        let king_s_b = BLOCKED_ROOK[king_idx]
            [pext(self.all_pieces, remove_border(EMPTY_PSEUDO_ROOK[king_idx])) as usize];
        let king_d_b = BLOCKED_BISHOP[king_idx][pext(
            self.all_pieces,
            remove_border(EMPTY_PSEUDO_BISHOP[king_idx]),
        ) as usize];
        let king_a_b = king_s_b | king_d_b;

        let (rooks, bishops, queens) = if white {
            (self.white.rook, self.white.bishop, self.white.queen)
        } else {
            (self.black.rook, self.black.bishop, self.black.queen)
        };

        let mut rook = rooks & king_straight;
        let mut bishop = bishops & king_diagonal;
        let mut queen = queens & king_all;

        let mut pins = Vec::new();

        while rook != 0 {
            let idx = rook.trailing_zeros() as usize;
            let moves = BLOCKED_ROOK[idx]
                [pext(self.all_pieces, remove_border(EMPTY_PSEUDO_ROOK[idx])) as usize];

            pins.push((moves & king_s_b).trailing_zeros() as usize);

            rook &= rook - 1;
        }
        while bishop != 0 {
            let idx = rook.trailing_zeros() as usize;
            let moves = BLOCKED_BISHOP[idx]
                [pext(self.all_pieces, remove_border(EMPTY_PSEUDO_BISHOP[idx])) as usize];

            pins.push((moves & king_d_b).trailing_zeros() as usize);

            bishop &= bishop - 1;
        }
        while queen != 0 {
            let idx = rook.trailing_zeros() as usize;
            let moves = BLOCKED_BISHOP[idx]
                [pext(self.all_pieces, remove_border(EMPTY_PSEUDO_BISHOP[idx])) as usize]
                | BLOCKED_ROOK[idx]
                    [pext(self.all_pieces, remove_border(EMPTY_PSEUDO_ROOK[idx])) as usize];

            pins.push((moves & king_a_b).trailing_zeros() as usize);

            queen &= queen - 1;
        }

        pins
    }

    pub fn get_legal_move_board(&self, pos_idx: u64, piece: PType, white: bool) -> (u64, u64) {
        let (board, king) = if white {
            (self.white.all, self.white.king)
        } else {
            (self.black.all, self.black.king)
        };

        let kings = self.white.king | self.black.king;

        fn move_board(
            state: &State,
            all: u64,
            board: u64,
            pos_idx: u64,
            piece: PType,
            white: bool,
            pawn_attacks: bool,
        ) -> u64 {
            match piece {
                PType::Pawn => state.pawn_moves(pos_idx, white, pawn_attacks),
                PType::Rook => {
                    BLOCKED_ROOK[pos_idx as usize][pext(
                        all,
                        remove_border_rook(EMPTY_PSEUDO_ROOK[pos_idx as usize], pos_idx as u8),
                    ) as usize]
                        & !board
                }
                PType::Knight => EMPTY_PSEUDO_KNIGHT[pos_idx as usize] & !board,
                PType::Bishop => {
                    BLOCKED_BISHOP[pos_idx as usize]
                        [pext(all, remove_border(EMPTY_PSEUDO_BISHOP[pos_idx as usize])) as usize]
                        & !board
                }
                PType::Queen => {
                    (BLOCKED_BISHOP[pos_idx as usize]
                        [pext(all, remove_border(EMPTY_PSEUDO_BISHOP[pos_idx as usize])) as usize]
                        & !board)
                        | (BLOCKED_ROOK[pos_idx as usize][pext(
                            all,
                            remove_border(EMPTY_PSEUDO_ROOK[pos_idx as usize]),
                        ) as usize]
                            & !board)
                }
                PType::King => panic!("Kings should be handled separately"),
            }
        }

        (
            move_board(self, self.all_pieces, board, pos_idx, piece, white, false),
            move_board(
                self,
                self.all_pieces & !kings,
                board & !king,
                pos_idx,
                piece,
                white,
                true,
            ),
        )
    }

    pub fn pawn_moves(&self, pos_idx: u64, white: bool, only_all_attacks: bool) -> u64 {
        let mut moves: u64 = 0;

        let start_row_after_1st_move: u64 = if white { 0xFF << 16 } else { 0xFF << (8 * 5) };

        let pos_board = 1 << pos_idx;

        let a: u64 = 0x0101010101010101;
        let h: u64 = a << 7;

        if only_all_attacks {
            return if white {
                ((pos_board & !a) << 7 | ((pos_board & !h) << 9))
            } else {
                ((pos_board & !h) >> 7 | ((pos_board & !a) >> 9))
            };
        }

        // attacks
        moves |= if white {
            ((pos_board & !a) << 7 | ((pos_board & !h) << 9)) & (self.black.all | self.en_passant)
        } else {
            ((pos_board & !h) >> 7 | ((pos_board & !a) >> 9)) & (self.white.all | self.en_passant)
        };

        // move one forward
        moves |= if white {
            pos_board << 8
        } else {
            pos_board >> 8
        } & !self.all_pieces;

        // two moves
        moves |= if white {
            (moves & start_row_after_1st_move) << 8 & !self.all_pieces
        } else {
            (moves & start_row_after_1st_move) >> 8 & !self.all_pieces
        };

        moves
    }

    pub fn get_all_moves(&self) -> (Vec<Move>, Vec<Move>) {
        let mut white_moves = Vec::new();
        let mut black_moves = Vec::new();

        let mut white_board_tot: u64 = 0;
        let mut black_board_tot: u64 = 0;

        let wpins = self.get_pins(true);
        let bpins = self.get_pins(false);

        for piece in &self.pieces_list {
            let pins = if piece.white { &wpins } else { &bpins };

            if pins.contains(&(piece.position_idx as usize)) {
                continue;
            }
            let (board, attacks) =
                self.get_legal_move_board(piece.position_idx, piece._type, piece.white);

            let moves = get_moves_from_move_board(board, piece.position_idx);

            if piece.white {
                white_moves.extend(moves);
                white_board_tot |= attacks;
            } else {
                black_moves.extend(moves);
                black_board_tot |= attacks;
            }
        }

        let widx = self.white.king.trailing_zeros() as u64;
        let bidx = self.black.king.trailing_zeros() as u64;

        let w_king_board =
            self.king_moves(widx, true) & !black_board_tot & !EMPTY_PSEUDO_KING[bidx as usize];
        let b_king_board =
            self.king_moves(bidx, false) & !white_board_tot & !EMPTY_PSEUDO_KING[widx as usize];

        white_moves.extend(get_moves_from_move_board(w_king_board, widx));
        black_moves.extend(get_moves_from_move_board(b_king_board, bidx));

        (white_moves, black_moves)
    }
}
