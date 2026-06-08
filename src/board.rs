use crate::*;

#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub white: bool,
    pub _type: PType,
}

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub white: ColorBoards,
    pub black: ColorBoards,
    pub all_pieces: u64,
    pub en_passant: u64,
    pub white_to_move: bool,
    pub half_move_clock: usize,
    pub full_move_clock: usize,
    pub pieces_list: [Option<Piece>; 64],
    pub pos_history: [u64; 100],
    pub history_idx: usize,
    pub hash: u64,
    pub endgame: f32,
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
    pub castled: i32,
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

impl Piece {
    pub fn score(&self) -> i32 {
        self._type.score()
    }
}

impl PType {
    pub fn score(&self) -> i32 {
        match self {
            PType::Pawn => 100,
            PType::Rook => 500,
            PType::Knight => 300,
            PType::Bishop => 300,
            PType::Queen => 900,
            PType::King => 0,
        }
    }
    pub fn sliding(&self) -> bool {
        match self {
            Self::Rook => true,
            Self::Bishop => true,
            Self::Queen => true,
            _ => false,
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            Self::Pawn => 0,
            Self::Knight => 1,
            Self::Bishop => 2,
            Self::Rook => 3,
            Self::Queen => 4,
            Self::King => 5,
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
    pub fn is_threefold_repetition(&self) -> bool {
        // if less than 4 moves it's impossible
        if self.history_idx < 4 {
            return false;
        }

        let mut repetitions = 1; // It appears once on the current board

        // We only need to check the history up to the current half_move_clock
        for i in 0..self.half_move_clock {
            if self.pos_history[i] == self.hash {
                repetitions += 1;
                if repetitions >= 3 {
                    return true;
                }
            }
        }

        false
    }

    pub fn calculate_initial_hash(&self) -> u64 {
        let mut hash = 0;

        // 1. Pieces
        for sq in 0..64 {
            if let Some(piece) = self.pieces_list[sq] {
                let color_idx = if piece.white { 0 } else { 1 };
                let piece_idx = piece._type.to_index();
                hash ^= ZOBRIST.pieces[color_idx][piece_idx][sq];
            }
        }

        // 2. Side to move
        if !self.white_to_move {
            hash ^= ZOBRIST.side_to_move;
        }

        // 3. Castling
        let mut castling_idx = 0;
        if self.white.castle_k {
            castling_idx |= 1;
        }
        if self.white.castle_q {
            castling_idx |= 2;
        }
        if self.black.castle_k {
            castling_idx |= 4;
        }
        if self.black.castle_q {
            castling_idx |= 8;
        }
        hash ^= ZOBRIST.castling[castling_idx];

        // 4. En Passant
        if self.en_passant != 0 {
            let ep_sq = self.en_passant.trailing_zeros() as usize;
            let file = ep_sq % 8; // We only need the file (A-H) for Zobrist
            hash ^= ZOBRIST.en_passant[file];
        }

        hash
    }
    pub fn evaluate(&self) -> i32 {
        if self.is_threefold_repetition() {
            // 0 for a draw
            return 0;
        }

        let mut white_count = 0;
        let mut black_count = 0;

        let number_attacking_white = (self.black.all & self.white.attacks).count_ones() as i32;
        let number_attacking_black = (self.white.all & self.black.attacks).count_ones() as i32;

        let number_defended_white =
            ((self.white.all & self.black.attacks) & self.white.attacks).count_ones() as i32;

        let number_defended_black =
            ((self.black.all & self.white.attacks) & self.black.attacks).count_ones() as i32;

        black_count += (number_attacking_black - number_defended_white) * 100;
        white_count += (number_attacking_white - number_defended_black) * 100;

        if self.white.attacks & self.black.king != 0 {
            white_count += 900
        }
        if self.black.attacks & self.white.king != 0 {
            black_count += 900
        }

        for (idx, p) in self.pieces_list.iter().enumerate() {
            if let Some(p) = p {
                let (count, position_score) = if p.white {
                    let pos_score = match p._type {
                        PType::Pawn => WHITE_PAWN[idx],
                        PType::Knight => WHITE_KNIGHT[idx],
                        PType::Rook => WHITE_ROOK[idx],
                        PType::Bishop => WHITE_BISHOP[idx],
                        PType::Queen => WHITE_QUEEN[idx],
                        PType::King => WHITE_KING[idx],
                    };

                    (&mut white_count, pos_score)
                } else {
                    let pos_score = match p._type {
                        PType::Pawn => BLACK_PAWN[idx],
                        PType::Knight => BLACK_KNIGHT[idx],
                        PType::Rook => BLACK_ROOK[idx],
                        PType::Bishop => BLACK_BISHOP[idx],
                        PType::Queen => BLACK_QUEEN[idx],
                        PType::King => BLACK_KING[idx],
                    };
                    (&mut black_count, pos_score)
                };
                *count += p.score() as i32 + position_score;
            }
        }

        white_count += self.white.castled;
        black_count += self.black.castled;

        if self.endgame > 0.6 {
            let endgame_white = king_to_corner_endgame(
                self.white.king.trailing_zeros() as usize,
                self.black.king.trailing_zeros() as usize,
                self.endgame,
            );

            let endgame_black = king_to_corner_endgame(
                self.black.king.trailing_zeros() as usize,
                self.white.king.trailing_zeros() as usize,
                self.endgame,
            );
            white_count += endgame_white;
            black_count += endgame_black;
        }

        let (own, other) = if self.white_to_move {
            (white_count, black_count)
        } else {
            (black_count, white_count)
        };

        own - other
    }

    pub fn update_all(&mut self) {
        self.white.update_all();
        self.black.update_all();
        self.all_pieces = self.white.all | self.black.all;
    }

    pub fn attackers_to(&self, pos_idx: u64, by_white: bool) -> u64 {
        let mut attackers = 0;

        let (pawns, knights, rooks, bishops, queens, kings) = if by_white {
            (
                self.white.pawn,
                self.white.knight,
                self.white.rook,
                self.white.bishop,
                self.white.queen,
                self.white.king,
            )
        } else {
            (
                self.black.pawn,
                self.black.knight,
                self.black.rook,
                self.black.bishop,
                self.black.queen,
                self.black.king,
            )
        };

        attackers |= EMPTY_PSEUDO_KNIGHT[pos_idx as usize] & knights;
        attackers |= EMPTY_PSEUDO_KING[pos_idx as usize] & kings;

        attackers |= self.pawn_moves(pos_idx, !by_white, true) & pawns;

        let straight_attacks = BLOCKED_ROOK[pos_idx as usize][pext(
            self.all_pieces,
            remove_border_rook(EMPTY_PSEUDO_ROOK[pos_idx as usize], pos_idx as u8),
        ) as usize];
        attackers |= straight_attacks & (rooks | queens);

        let diagonal_attacks = BLOCKED_BISHOP[pos_idx as usize][pext(
            self.all_pieces,
            remove_border(EMPTY_PSEUDO_BISHOP[pos_idx as usize]),
        ) as usize];
        attackers |= diagonal_attacks & (bishops | queens);

        attackers
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

        // remove old castling rights
        let mut old_castling_idx = 0;
        if self.white.castle_k {
            old_castling_idx |= 1;
        }
        if self.white.castle_q {
            old_castling_idx |= 2;
        }
        if self.black.castle_k {
            old_castling_idx |= 4;
        }
        if self.black.castle_q {
            old_castling_idx |= 8;
        }
        self.hash ^= ZOBRIST.castling[old_castling_idx];

        // add new ones
        match from {
            0 => self.white.castle_q = false,
            4 => {
                self.white.castle_q = false;
                self.white.castle_k = false;
            }
            7 => self.white.castle_k = false,
            56 => self.black.castle_q = false,
            60 => {
                self.black.castle_q = false;
                self.black.castle_k = false;
            }
            63 => self.black.castle_k = false,
            _ => {}
        }

        let mut castling_idx = 0;
        if self.white.castle_k {
            castling_idx |= 1;
        }
        if self.white.castle_q {
            castling_idx |= 2;
        }
        if self.black.castle_k {
            castling_idx |= 4;
        }
        if self.black.castle_q {
            castling_idx |= 8;
        }
        self.hash ^= ZOBRIST.castling[castling_idx];

        let p = self.pieces_list[from];

        let (color_board, other_color) = if self.white_to_move {
            (&mut self.white, &mut self.black)
        } else {
            (&mut self.black, &mut self.white)
        };

        // remove old en passant
        if self.en_passant != 0 {
            let ep_sq = self.en_passant.trailing_zeros() as usize;
            self.hash ^= ZOBRIST.en_passant[ep_sq % 8];
        }

        let mut reset_clock = false;
        let old_en_passant = self.en_passant;
        self.en_passant = 0;

        if let Some(mut piece) = p {
            // not king moves

            // xor the from piece
            self.hash ^= ZOBRIST.pieces[self.white_to_move as usize][piece._type.to_index()][from];

            if to == old_en_passant.trailing_zeros() as usize && piece._type == PType::Pawn {
                // remove the old en passant pawn
                fn shift(white: bool, x: usize) -> usize {
                    if white {
                        x - 8
                    } else {
                        x + 8
                    }
                }

                self.pieces_list[shift(self.white_to_move, to)] = None;
                other_color.pawn &= !(1 << shift(self.white_to_move, to));

                // xor the old pawn
                self.hash ^= ZOBRIST.pieces[!self.white_to_move as usize][PType::Pawn.to_index()]
                    [shift(self.white_to_move, to)];
            }

            if piece._type == PType::Pawn {
                reset_clock = true;
                if prom.is_some() {
                    piece._type = prom.unwrap();
                }

                // if moves two forward update en_passant
                if from.abs_diff(to) == 16 {
                    self.en_passant = 1 << (from + to) / 2;
                    let ep_sq = self.en_passant.trailing_zeros() as usize;
                    self.hash ^= ZOBRIST.en_passant[ep_sq % 8];
                }
            }

            // remove the take piece
            if let Some(p) = self.pieces_list[to] {
                self.hash ^= ZOBRIST.pieces[!self.white_to_move as usize][p._type.to_index()][to];

                reset_clock = true;
                // capture happened
                // remove the taken piece (not checking which one it was)
                other_color.pawn &= !(1 << to);
                other_color.rook &= !(1 << to);
                other_color.knight &= !(1 << to);
                other_color.bishop &= !(1 << to);
                other_color.queen &= !(1 << to);
            }

            self.pieces_list[to] = Some(piece);
            self.pieces_list[from] = None;

            // if there was a promotion, remove the old pawn (because the piece type is now not a
            // pawn anymore)
            if prom.is_some() {
                color_board.pawn &= !(1 << from as u64);
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

            // add the to piece
            self.hash ^= ZOBRIST.pieces[self.white_to_move as usize][piece._type.to_index()][to];
        } else {
            // king moves
            //remove the king
            self.hash ^= ZOBRIST.pieces[self.white_to_move as usize][PType::King.to_index()][from];

            // if caslte
            if from.abs_diff(to) == 2 {
                let shift = if self.white_to_move { 0 } else { 56 };

                color_board.castled = 5;

                if from < to {
                    // king-side
                    color_board.king = 1 << to as u64;
                    color_board.rook &= !(0b10000000 << shift);
                    color_board.rook |= 0b100000 << shift;

                    let rook = self.pieces_list[7 + shift];
                    self.pieces_list[5 + shift] = rook;
                    self.pieces_list[7 + shift] = None;

                    // remove the rook
                    self.hash ^= ZOBRIST.pieces[self.white_to_move as usize]
                        [PType::Rook.to_index()][shift + 7];
                    //add the rook
                    self.hash ^= ZOBRIST.pieces[self.white_to_move as usize]
                        [PType::Rook.to_index()][shift + 5];
                } else {
                    // queen-side
                    color_board.king = 1 << to as u64;
                    color_board.rook &= !(0b1 << shift);
                    color_board.rook |= 0b1000 << shift;

                    let rook = self.pieces_list[shift];
                    self.pieces_list[3 + shift] = rook;
                    self.pieces_list[shift] = None;

                    // remove the rook
                    self.hash ^=
                        ZOBRIST.pieces[self.white_to_move as usize][PType::Rook.to_index()][shift];
                    //add the rook
                    self.hash ^= ZOBRIST.pieces[self.white_to_move as usize]
                        [PType::Rook.to_index()][shift + 3];
                }
            } else {
                color_board.king = 1 << to as u64;

                // remove the taken piece
                if let Some(p) = self.pieces_list[to] {
                    self.hash ^=
                        ZOBRIST.pieces[!self.white_to_move as usize][p._type.to_index()][to];

                    reset_clock = true;
                    // capture happened
                    // remove the taken piece (not checking which one it was)
                    other_color.pawn &= !(1 << to);
                    other_color.rook &= !(1 << to);
                    other_color.knight &= !(1 << to);
                    other_color.bishop &= !(1 << to);
                    other_color.queen &= !(1 << to);
                }

                self.hash ^=
                    ZOBRIST.pieces[self.white_to_move as usize][PType::King.to_index()][to];

                self.pieces_list[to] = None;
            }
        }

        if !self.white_to_move {
            self.full_move_clock += 1;
        }
        self.update_all();
        self.update_endgame();
        self.white_to_move = !self.white_to_move;
        self.hash ^= ZOBRIST.side_to_move;

        if reset_clock {
            self.pos_history = [0; 100];
            self.history_idx = 0;
            self.pos_history[self.history_idx] = self.hash;
            self.half_move_clock = 0
        } else {
            self.half_move_clock += 1;
            self.history_idx += 1;
            self.pos_history[self.history_idx] = self.hash;
        }
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
        let endgame = 1. - ((pieces.len() as f32 - 3.).max(0.) / 30.);

        Self {
            white,
            black,
            en_passant,
            white_to_move,
            all_pieces,
            half_move_clock,
            full_move_clock,
            pieces_list: pieces,
            pos_history: [0; 100],
            history_idx: 0,
            hash: 0,
            endgame,
        }
    }
    pub fn update_endgame(&mut self) {
        let endgame = 1. - ((self.all_pieces.count_ones() as f32 - 5.).max(0.) / 30.);
        self.endgame = endgame;
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
            remove_border_rook(king_straight, king_idx as u8),
        ) as usize];
        let king_d_b =
            BLOCKED_BISHOP[king_idx][pext(self.all_pieces, remove_border(king_diagonal)) as usize];
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
            rook &= rook - 1;
        }
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
                | BLOCKED_ROOK[idx][pext(
                    self.all_pieces,
                    remove_border_rook(EMPTY_PSEUDO_ROOK[idx], idx as u8),
                ) as usize];

            pins.push((moves & king_a_b & RAY_BETWEEN.0[king_idx][idx]).trailing_zeros() as usize);

            queen &= queen - 1;
        }

        pins
    }

    pub fn get_legal_move_board(&self, pos_idx: u64, piece: PType, white: bool) -> (u64, u64) {
        let (board, king) = if white {
            (self.white.all, self.black.king)
        } else {
            (self.black.all, self.white.king)
        };

        fn move_board(
            state: &State,
            all: u64,
            board: u64,
            pos_idx: u64,
            piece: PType,
            white: bool,
            getting_attacks: bool,
        ) -> u64 {
            let mut attacks = match piece {
                PType::Pawn => state.pawn_moves(pos_idx, white, getting_attacks),
                PType::Rook => {
                    BLOCKED_ROOK[pos_idx as usize][pext(
                        all,
                        remove_border_rook(EMPTY_PSEUDO_ROOK[pos_idx as usize], pos_idx as u8),
                    ) as usize]
                }
                PType::Knight => EMPTY_PSEUDO_KNIGHT[pos_idx as usize],
                PType::Bishop => {
                    BLOCKED_BISHOP[pos_idx as usize]
                        [pext(all, remove_border(EMPTY_PSEUDO_BISHOP[pos_idx as usize])) as usize]
                }
                PType::Queen => {
                    (BLOCKED_BISHOP[pos_idx as usize]
                        [pext(all, remove_border(EMPTY_PSEUDO_BISHOP[pos_idx as usize])) as usize])
                        | (BLOCKED_ROOK[pos_idx as usize][pext(
                            all,
                            remove_border_rook(EMPTY_PSEUDO_ROOK[pos_idx as usize], pos_idx as u8),
                        ) as usize])
                }
                PType::King => panic!("Kings should be handled separately"),
            };
            if getting_attacks {
                attacks
            } else {
                attacks & !board
            }
        }

        (
            move_board(self, self.all_pieces, board, pos_idx, piece, white, false),
            move_board(
                self,
                self.all_pieces & !king,
                board,
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

        // attacks
        moves |= if white {
            ((pos_board & !a) << 7 | ((pos_board & !h) << 9)) & (self.black.all | self.en_passant)
        } else {
            ((pos_board & !h) >> 7 | ((pos_board & !a) >> 9)) & (self.white.all | self.en_passant)
        };

        moves
    }

    pub fn get_moves(&mut self) -> Vec<Move> {
        let mut moves = Vec::new();
        let mut all_w_attacks: u64 = 0;
        let mut all_b_attacks: u64 = 0;
        let pins = self.get_pins(self.white_to_move);

        let king_idx = if self.white_to_move {
            self.white.king.trailing_zeros() as u64
        } else {
            self.black.king.trailing_zeros() as u64
        };

        let attackers = self.attackers_to(king_idx, !self.white_to_move);
        let num_attackers = attackers.count_ones();
        let mut check_mask = !0u64;

        if num_attackers == 1 {
            let attacker_idx = attackers.trailing_zeros() as u64;
            check_mask = 1 << attacker_idx;

            check_mask |= RAY_BETWEEN.0[king_idx as usize][attacker_idx as usize];
        }

        let (c_board, other) = if self.white_to_move {
            (self.white, self.black)
        } else {
            (self.black, self.white)
        };

        for (idx, piece) in self.pieces_list.iter().enumerate() {
            let piece = if let Some(p) = piece {
                p
            } else {
                continue;
            };

            let (mut board, attacks) =
                self.get_legal_move_board(idx as u64, piece._type, piece.white);

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

            // only king move helps
            if piece.white == self.white_to_move {
                if num_attackers > 1 {
                    continue;
                }

                //only moves that can block it
                board &= check_mask;
            }

            if pins.contains(&idx) {
                board &= RAY_BETWEEN.1[idx][c_board.king.trailing_zeros() as usize];
            }

            let mut m =
                get_moves_from_move_board(board, idx as u64, piece._type, self.white_to_move);

            moves.extend(m);
        }

        all_w_attacks |= EMPTY_PSEUDO_KING[self.white.king.trailing_zeros() as usize];
        all_b_attacks |= EMPTY_PSEUDO_KING[self.black.king.trailing_zeros() as usize];

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
