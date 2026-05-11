use crate::*;

#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub white: bool,
    pub _type: PType,
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
    pub pieces_list: [Option<Piece>; 64],
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
    pub attacks: u64,
    pub castle_k: bool,
    pub castle_q: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PType {
    Pawn,
    Rook,
    Knight,
    Bishop,
    King,
    Queen,
}

// 0000-000000-000000
// prom   to    from
pub type Move = u16;

pub fn get_moves_from_move_board(
    mut move_board: u64,
    idx: u64,
    _type: PType,
    white_move: bool,
) -> Vec<Move> {
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
        let last_row = if white_move { index >= 56 } else { index <= 7 };
        let promotion = _type == PType::Pawn && last_row;

        let from = idx as u16;
        let to = (index as u16) << 6;

        if promotion {
            for p in [
                PROMOTION_QUEEN,
                PROMOTION_BISHOP,
                PROMOTION_KNIGHT,
                PROMOTION_ROOK,
            ] {
                moves.push(p | to | from)
            }

            continue;
        }

        moves.push(to | from);
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
    pub fn update_all(&mut self) {
        self.white.update_all();
        self.black.update_all();
        self.all_pieces = self.white.all | self.black.all;
    }
    pub fn make_move(&mut self, _move: Move) {
        let from = _move as usize & 0b111111;
        let to = (_move as usize & (0b111111 << 6)) >> 6;
        let prom = match _move & (0b1111 << 12) {
            PROMOTION_QUEEN => Some(PType::Queen),
            PROMOTION_KNIGHT => Some(PType::Knight),
            PROMOTION_BISHOP => Some(PType::Bishop),
            PROMOTION_ROOK => Some(PType::Rook),
            _ => None,
        };

        let p = self.pieces_list[from];

        let (color_board, other_color) = if self.white_to_move {
            (&mut self.white, &mut self.black)
        } else {
            (&mut self.black, &mut self.white)
        };

        self.en_passant = 0;

        let mut reset_clock = false;

        if let Some(mut piece) = p {
            // not king moves

            if piece._type == PType::Pawn {
                reset_clock = true;
                if prom.is_some() {
                    piece._type = prom.unwrap();
                    // if moves two forward update en_passant
                    if from.abs_diff(to) == 16 {
                        self.en_passant = 1 << (from + to) / 2
                    }
                }
            }

            self.pieces_list[to] = Some(piece);
            self.pieces_list[from] = None;

            if (other_color.pawn
                | other_color.rook
                | other_color.knight
                | other_color.bishop
                | other_color.queen)
                & 1 << to
                != 0
            {
                reset_clock = true;
                // capture happened
                // remove the taken piece
                other_color.pawn &= !(1 << to);
                other_color.rook &= !(1 << to);
                other_color.knight &= !(1 << to);
                other_color.bishop &= !(1 << to);
                other_color.queen &= !(1 << to);
            }

            // get the to-piece board
            // add the correct piece to the board
            match piece._type {
                PType::Pawn => {
                    color_board.pawn &= !(1 << from as u64);
                    color_board.pawn |= 1 << to as u64;
                }
                PType::Rook => {
                    color_board.rook &= !(1 << from as u64);
                    color_board.rook |= 1 << to as u64;
                }
                PType::Bishop => {
                    color_board.bishop &= !(1 << from as u64);
                    color_board.bishop |= 1 << to as u64;
                }
                PType::Knight => {
                    color_board.knight &= !(1 << from as u64);
                    color_board.knight |= 1 << to as u64;
                }
                PType::Queen => {
                    color_board.queen &= !(1 << from as u64);
                    color_board.queen |= 1 << to as u64;
                }
                PType::King => panic!("Kings are not supposed to exist here"),
            }
        } else {
            // king moves
            if from.abs_diff(to) == 2 {
                // if caslte
                let shift = if self.white_to_move { 0 } else { 56 };
                if from < to {
                    // king-side
                    color_board.king = 1 << to as u64;
                    color_board.rook &= !(0b10000000 << shift);
                    color_board.rook |= 0b100000 << shift;
                } else {
                    // queen-side
                    color_board.king = 1 << to as u64;
                    color_board.rook &= !(0b1 << shift);
                    color_board.rook |= 0b1000 << shift;
                }
            } else {
                color_board.king = 1 << to as u64;
                self.pieces_list[to] = None;
            }
        }

        if reset_clock {
            self.half_move_clock = 0
        } else {
            self.half_move_clock += 1;
        }
        if !self.white_to_move {
            self.full_move_clock += 1;
        }
        self.update_all();
        self.white_to_move = !self.white_to_move;
    }

    pub fn from_fen(fen: &str) -> Self {
        let mut pieces: [Option<Piece>; 64] = [None; 64];

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

        let (white, black) = fen_positions_to_bitboards(pos, &mut pieces, cast);
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

        let king_s_b = BLOCKED_ROOK[king_idx][pext(
            self.all_pieces,
            remove_border_rook(EMPTY_PSEUDO_ROOK[king_idx], king_idx as u8),
        ) as usize];
        let king_d_b = BLOCKED_BISHOP[king_idx][pext(
            self.all_pieces,
            remove_border(EMPTY_PSEUDO_BISHOP[king_idx]),
        ) as usize];
        let king_a_b = king_s_b | king_d_b;

        let (rooks, bishops, queens) = if white {
            (self.black.rook, self.black.bishop, self.black.queen)
        } else {
            (self.white.rook, self.white.bishop, self.white.queen)
        };

        let mut rook = rooks & king_straight;
        let mut bishop = bishops & king_diagonal;
        let mut queen = queens & king_all;

        let mut pins = Vec::new();

        while rook != 0 {
            let idx = rook.trailing_zeros() as usize;
            let moves = BLOCKED_ROOK[idx][pext(
                self.all_pieces,
                remove_border_rook(EMPTY_PSEUDO_ROOK[idx], idx as u8),
            ) as usize];

            if moves & king_s_b != 0 {
                pins.push((moves & king_s_b).trailing_zeros() as usize);
            }
        }
        rook &= rook - 1;
        while bishop != 0 {
            let idx = bishop.trailing_zeros() as usize;
            let moves = BLOCKED_BISHOP[idx]
                [pext(self.all_pieces, remove_border(EMPTY_PSEUDO_BISHOP[idx])) as usize];

            pins.push((moves & king_d_b).trailing_zeros() as usize);

            bishop &= bishop - 1;
        }
        while queen != 0 {
            let idx = queen.trailing_zeros() as usize;
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
                            remove_border_rook(EMPTY_PSEUDO_ROOK[pos_idx as usize], pos_idx as u8),
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

    pub fn get_moves(&mut self) -> Vec<Move> {
        let mut moves = Vec::new();
        let mut all_w_attacks: u64 = 0;
        let mut all_b_attacks: u64 = 0;
        let pins = self.get_pins(self.white_to_move);

        for (idx, piece) in self.pieces_list.iter().enumerate() {
            let piece = if let Some(p) = piece {
                p
            } else {
                continue;
            };

            let (board, attacks) = self.get_legal_move_board(idx as u64, piece._type, piece.white);

            if piece.white {
                all_w_attacks |= attacks;
                if !self.white_to_move {
                    continue;
                }
            } else {
                all_b_attacks |= attacks;
                if self.white_to_move {
                    continue;
                }
            }

            if pins.contains(&idx) {
                continue;
            }

            let _move =
                get_moves_from_move_board(board, idx as u64, piece._type, self.white_to_move);

            moves.extend(_move);
        }

        self.white.attacks = all_w_attacks;
        self.black.attacks = all_b_attacks;

        let (idx, other_king) = if self.white_to_move {
            (
                self.white.king.trailing_zeros() as u64,
                self.black.king.trailing_zeros() as usize,
            )
        } else {
            (
                self.black.king.trailing_zeros() as u64,
                self.white.king.trailing_zeros() as usize,
            )
        };

        let other_attacks = if self.white_to_move {
            self.black.attacks
        } else {
            self.white.attacks
        };

        let king_board = self.king_moves(idx, self.white_to_move)
            & !other_attacks
            & !EMPTY_PSEUDO_KING[other_king];

        moves.extend(get_moves_from_move_board(
            king_board,
            idx,
            PType::King,
            self.white_to_move,
        ));

        let (king, queen) = self.get_castling();

        if king {
            let _move = if self.white_to_move {
                WHITE_KING_CASTLE
            } else {
                BLACK_KING_CASTLE
            };
            moves.push(_move)
        }
        if queen {
            let _move = if self.white_to_move {
                WHITE_QUEEN_CASTLE
            } else {
                BLACK_QUEEN_CASTLE
            };
            moves.push(_move)
        }

        moves
    }

    pub fn get_castling(&self) -> (bool, bool) {
        let (board, other, shift) = if self.white_to_move {
            (self.white, self.black, 0)
        } else {
            (self.black, self.white, 56)
        };

        let mut castle = (false, false);

        if board.castle_k {
            if (0b1110000 << shift) & other.attacks == 0
                && (0b1100000 << shift) & self.all_pieces == 0
            {
                castle.0 = true
            }
        }
        if board.castle_q {
            if (0b1100 << shift) & other.attacks == 0 && (0b1110 << shift) & self.all_pieces == 0 {
                castle.1 = true
            }
        }

        castle
    }
}
