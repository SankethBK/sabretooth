use crate::attacks::{attack_table, bishop_attacks, queen_attacks, rook_attacks};
use crate::bitboard::pop_lsb;
use crate::types::{CastlingRights, Color, Move, MoveKind, Piece, Square};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FenError {
    InvalidFieldCount,
    InvalidPiecePlacement,
    InvalidSideToMove,
    InvalidCastlingRights,
    InvalidEnPassant,
    InvalidHalfmoveClock,
    InvalidFullmoveNumber,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HistoryEntry {
    mv: Move,
    captured: Option<Piece>,
    castling_rights: u8,
    en_passant: Option<Square>,
    halfmove_clock: u32,
    fullmove_number: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    pieces: [[u64; 6]; 2],
    pub(crate) side_to_move: Color,
    castling_rights: u8,
    pub(crate) en_passant: Option<Square>,
    halfmove_clock: u32,
    fullmove_number: u32,

    history: Vec<HistoryEntry>,
}

impl Board {
    pub fn new() -> Self {
        Board {
            pieces: [[0; 6]; 2],
            side_to_move: Color::White,
            castling_rights: CastlingRights::NONE,
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            history: Vec::new(),
        }
    }

    pub fn from_fen(fen: &str) -> Result<Board, FenError> {
        let mut board = Board::new();
        let fields: Vec<&str> = fen.split_whitespace().collect();
        if fields.len() != 6 {
            return Err(FenError::InvalidFieldCount);
        }

        let placement = fields[0];
        let side_to_move = fields[1];
        let castling = fields[2];
        let en_passant = fields[3];
        let halfmove = fields[4];
        let fullmove = fields[5];

        let mut rank: i32 = 7;
        let mut file: i32 = 0;

        for ch in placement.chars() {
            match ch {
                '/' => {
                    if file != 8 || rank == 0 {
                        return Err(FenError::InvalidPiecePlacement);
                    }
                    rank -= 1;
                    file = 0;
                }
                '1'..='8' => {
                    file += (ch as u8 - b'0') as i32;
                    if file > 8 {
                        return Err(FenError::InvalidPiecePlacement);
                    }
                }
                _ => {
                    let Some((color, piece)) = parse_piece_char(ch) else {
                        return Err(FenError::InvalidPiecePlacement);
                    };

                    if file >= 8 || rank < 0 {
                        return Err(FenError::InvalidPiecePlacement);
                    }

                    let square = Square::from_rf(rank as u8, file as u8);
                    board.put_piece(square, color, piece);
                    file += 1;
                }
            }
        }

        if rank != 0 || file != 8 {
            return Err(FenError::InvalidPiecePlacement);
        }

        board.side_to_move = match side_to_move {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(FenError::InvalidSideToMove),
        };

        board.castling_rights = parse_castling_rights(castling)?;
        board.en_passant = parse_en_passant(en_passant)?;
        board.halfmove_clock = halfmove
            .parse::<u32>()
            .map_err(|_| FenError::InvalidHalfmoveClock)?;
        board.fullmove_number = fullmove
            .parse::<u32>()
            .map_err(|_| FenError::InvalidFullmoveNumber)?;

        if board.fullmove_number == 0 {
            return Err(FenError::InvalidFullmoveNumber);
        }

        Ok(board)
    }

    pub fn starting_position() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .expect("starting position FEN must be valid")
    }

    pub fn piece_bb(&self, color: Color, piece: Piece) -> u64 {
        self.pieces[color.index()][piece.index()]
    }

    pub fn occupancy(&self, color: Color) -> u64 {
        self.pieces[color.index()]
            .iter()
            .copied()
            .fold(0, |acc, bb| acc | bb)
    }

    pub fn all_occupancy(&self) -> u64 {
        self.occupancy(Color::White) | self.occupancy(Color::Black)
    }

    pub fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    pub fn castling_rights(&self) -> u8 {
        self.castling_rights
    }

    pub fn en_passant(&self) -> Option<Square> {
        self.en_passant
    }

    pub fn halfmove_clock(&self) -> u32 {
        self.halfmove_clock
    }

    pub fn fullmove_number(&self) -> u32 {
        self.fullmove_number
    }

    pub fn piece_on(&self, square: Square) -> Option<(Color, Piece)> {
        let mask = square.to_bb();

        for color in [Color::White, Color::Black] {
            for piece in [
                Piece::Pawn,
                Piece::Knight,
                Piece::Bishop,
                Piece::Rook,
                Piece::Queen,
                Piece::King,
            ] {
                if self.pieces[color.index()][piece.index()] & mask != 0 {
                    return Some((color, piece));
                }
            }
        }

        None
    }

    pub fn is_square_attacked(&self, square: Square, by: Color) -> bool {
        let attacks = attack_table();
        let target = square.to_bb();
        let occupancy = self.all_occupancy();

        let mut pawns = self.piece_bb(by, Piece::Pawn);
        while pawns != 0 {
            let from = pop_lsb(&mut pawns);
            if attacks.pawn[by.index()][from.index()] & target != 0 {
                return true;
            }
        }

        let mut knights = self.piece_bb(by, Piece::Knight);
        while knights != 0 {
            let from = pop_lsb(&mut knights);
            if attacks.knight[from.index()] & target != 0 {
                return true;
            }
        }

        let mut bishops = self.piece_bb(by, Piece::Bishop);
        while bishops != 0 {
            let from = pop_lsb(&mut bishops);
            if bishop_attacks(from, occupancy) & target != 0 {
                return true;
            }
        }

        let mut rooks = self.piece_bb(by, Piece::Rook);
        while rooks != 0 {
            let from = pop_lsb(&mut rooks);
            if rook_attacks(from, occupancy) & target != 0 {
                return true;
            }
        }

        let mut queens = self.piece_bb(by, Piece::Queen);
        while queens != 0 {
            let from = pop_lsb(&mut queens);
            if queen_attacks(from, occupancy) & target != 0 {
                return true;
            }
        }

        let mut kings = self.piece_bb(by, Piece::King);
        while kings != 0 {
            let from = pop_lsb(&mut kings);
            if attacks.king[from.index()] & target != 0 {
                return true;
            }
        }

        false
    }

    pub fn pretty_print(&self) {
        println!();
        for rank in (0..8).rev() {
            print!("  {} ", rank + 1);
            for file in 0..8 {
                let sq = Square::from_rf(rank, file);
                // checkerboard background using spaces
                let is_light = (rank + file) % 2 == 0;
                let symbol = match self.piece_on(sq) {
                    Some((Color::White, Piece::King)) => "♔",
                    Some((Color::White, Piece::Queen)) => "♕",
                    Some((Color::White, Piece::Rook)) => "♖",
                    Some((Color::White, Piece::Bishop)) => "♗",
                    Some((Color::White, Piece::Knight)) => "♘",
                    Some((Color::White, Piece::Pawn)) => "♙",
                    Some((Color::Black, Piece::King)) => "♚",
                    Some((Color::Black, Piece::Queen)) => "♛",
                    Some((Color::Black, Piece::Rook)) => "♜",
                    Some((Color::Black, Piece::Bishop)) => "♝",
                    Some((Color::Black, Piece::Knight)) => "♞",
                    Some((Color::Black, Piece::Pawn)) => "♟",
                    None => {
                        if is_light {
                            "·"
                        } else {
                            " "
                        }
                    }
                };
                print!("{} ", symbol);
            }
            println!();
        }
        println!("    a b c d e f g h");
        println!();
        println!("  Side to move: {:?}", self.side_to_move);
        println!("  Castling:     {:04b}", self.castling_rights);
        println!("  En passant:   {:?}", self.en_passant);
        println!("  Halfmove:     {}", self.halfmove_clock);
        println!("  Fullmove:     {}", self.fullmove_number);
        println!();
    }

    fn make_move(&mut self, mv: Move) {
        let us = self.side_to_move;
        let them = us.opposite();

        let (color, moving_piece) = self
            .piece_on(mv.from)
            .expect("make_move requires a piece on the from square");
        assert_eq!(color, us, "moving side must match side_to_move");

        let captured_square = match mv.kind {
            MoveKind::EnPassant => Some(match us {
                Color::White => Square::from_rf(mv.to.rank() - 1, mv.to.file()),
                Color::Black => Square::from_rf(mv.to.rank() + 1, mv.to.file()),
            }),
            MoveKind::Capture | MoveKind::PromotionCapture(_) => Some(mv.to),
            _ => None,
        };

        let captured_piece =
            captured_square.and_then(|sq| self.piece_on(sq).map(|(_, piece)| piece));

        self.history.push(HistoryEntry {
            mv,
            captured: captured_piece,
            castling_rights: self.castling_rights,
            en_passant: self.en_passant,
            halfmove_clock: self.halfmove_clock,
            fullmove_number: self.fullmove_number,
        });

        self.en_passant = None;

        if let Some(square) = captured_square {
            if let Some(piece) = captured_piece {
                self.remove_piece(square, them, piece);
                self.clear_castling_rights_for_rook_square(square);
            }
        }

        match mv.kind {
            MoveKind::Quiet | MoveKind::Capture | MoveKind::DoublePawnPush => {
                self.move_piece(mv.from, mv.to, us, moving_piece);
            }
            MoveKind::Promotion(promoted) | MoveKind::PromotionCapture(promoted) => {
                self.remove_piece(mv.from, us, Piece::Pawn);
                self.put_piece(mv.to, us, promoted);
            }
            MoveKind::EnPassant => {
                self.move_piece(mv.from, mv.to, us, Piece::Pawn);
            }
            MoveKind::CastleKingside => {
                self.move_piece(mv.from, mv.to, us, Piece::King);
                match us {
                    Color::White => self.move_piece(
                        Square::from_rf(0, 7),
                        Square::from_rf(0, 5),
                        Color::White,
                        Piece::Rook,
                    ),
                    Color::Black => self.move_piece(
                        Square::from_rf(7, 7),
                        Square::from_rf(7, 5),
                        Color::Black,
                        Piece::Rook,
                    ),
                }
            }
            MoveKind::CastleQueenside => {
                self.move_piece(mv.from, mv.to, us, Piece::King);
                match us {
                    Color::White => self.move_piece(
                        Square::from_rf(0, 0),
                        Square::from_rf(0, 3),
                        Color::White,
                        Piece::Rook,
                    ),
                    Color::Black => self.move_piece(
                        Square::from_rf(7, 0),
                        Square::from_rf(7, 3),
                        Color::Black,
                        Piece::Rook,
                    ),
                }
            }
        }

        match moving_piece {
            Piece::King => match us {
                Color::White => {
                    self.castling_rights &=
                        !(CastlingRights::WHITE_KINGSIDE | CastlingRights::WHITE_QUEENSIDE)
                }
                Color::Black => {
                    self.castling_rights &=
                        !(CastlingRights::BLACK_KINGSIDE | CastlingRights::BLACK_QUEENSIDE)
                }
            },
            Piece::Rook => self.clear_castling_rights_for_rook_square(mv.from),
            _ => {}
        }

        if matches!(mv.kind, MoveKind::DoublePawnPush) {
            self.en_passant = Some(match us {
                Color::White => Square::from_rf(mv.from.rank() + 1, mv.from.file()),
                Color::Black => Square::from_rf(mv.from.rank() - 1, mv.from.file()),
            });
        }

        if moving_piece == Piece::Pawn || mv.is_capture() {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        if us == Color::Black {
            self.fullmove_number += 1;
        }

        self.side_to_move = them;
    }

    fn unmake_move(&mut self) {
        let entry = self.history.pop().expect("unmake_move requires prior history");
        let mv = entry.mv;

        self.side_to_move = self.side_to_move.opposite();
        let us = self.side_to_move;
        let them = us.opposite();

        self.castling_rights = entry.castling_rights;
        self.en_passant = entry.en_passant;
        self.halfmove_clock = entry.halfmove_clock;
        self.fullmove_number = entry.fullmove_number;

        match mv.kind {
            MoveKind::Quiet | MoveKind::DoublePawnPush => {
                let piece = self
                    .piece_on(mv.to)
                    .map(|(_, piece)| piece)
                    .expect("quiet move destination must contain moving piece");
                self.move_piece(mv.to, mv.from, us, piece);
            }
            MoveKind::Capture => {
                let piece = self
                    .piece_on(mv.to)
                    .map(|(_, piece)| piece)
                    .expect("capture destination must contain moving piece");
                self.move_piece(mv.to, mv.from, us, piece);
                let captured = entry
                    .captured
                    .expect("capture history must record captured piece");
                self.put_piece(mv.to, them, captured);
            }
            MoveKind::EnPassant => {
                self.move_piece(mv.to, mv.from, us, Piece::Pawn);
                let captured_square = match us {
                    Color::White => Square::from_rf(mv.to.rank() - 1, mv.to.file()),
                    Color::Black => Square::from_rf(mv.to.rank() + 1, mv.to.file()),
                };
                self.put_piece(captured_square, them, Piece::Pawn);
            }
            MoveKind::Promotion(_) => {
                let promoted = self
                    .piece_on(mv.to)
                    .map(|(_, piece)| piece)
                    .expect("promotion destination must contain promoted piece");
                self.remove_piece(mv.to, us, promoted);
                self.put_piece(mv.from, us, Piece::Pawn);
            }
            MoveKind::PromotionCapture(_) => {
                let promoted = self
                    .piece_on(mv.to)
                    .map(|(_, piece)| piece)
                    .expect("promotion capture destination must contain promoted piece");
                self.remove_piece(mv.to, us, promoted);
                self.put_piece(mv.from, us, Piece::Pawn);
                let captured = entry
                    .captured
                    .expect("promotion capture history must record captured piece");
                self.put_piece(mv.to, them, captured);
            }
            MoveKind::CastleKingside => match us {
                Color::White => {
                    self.move_piece(mv.to, mv.from, Color::White, Piece::King);
                    self.move_piece(
                        Square::from_rf(0, 5),
                        Square::from_rf(0, 7),
                        Color::White,
                        Piece::Rook,
                    );
                }
                Color::Black => {
                    self.move_piece(mv.to, mv.from, Color::Black, Piece::King);
                    self.move_piece(
                        Square::from_rf(7, 5),
                        Square::from_rf(7, 7),
                        Color::Black,
                        Piece::Rook,
                    );
                }
            },
            MoveKind::CastleQueenside => match us {
                Color::White => {
                    self.move_piece(mv.to, mv.from, Color::White, Piece::King);
                    self.move_piece(
                        Square::from_rf(0, 3),
                        Square::from_rf(0, 0),
                        Color::White,
                        Piece::Rook,
                    );
                }
                Color::Black => {
                    self.move_piece(mv.to, mv.from, Color::Black, Piece::King);
                    self.move_piece(
                        Square::from_rf(7, 3),
                        Square::from_rf(7, 0),
                        Color::Black,
                        Piece::Rook,
                    );
                }
            },
        }
    }

    fn put_piece(&mut self, square: Square, color: Color, piece: Piece) {
        self.pieces[color.index()][piece.index()] |= square.to_bb();
    }

    fn remove_piece(&mut self, square: Square, color: Color, piece: Piece) {
        self.pieces[color.index()][piece.index()] &= !square.to_bb();
    }

    fn move_piece(&mut self, from: Square, to: Square, color: Color, piece: Piece) {
        self.remove_piece(from, color, piece);
        self.put_piece(to, color, piece);
    }

    fn clear_castling_rights_for_rook_square(&mut self, square: Square) {
        match square {
            Square(0) => self.castling_rights &= !CastlingRights::WHITE_QUEENSIDE,
            Square(7) => self.castling_rights &= !CastlingRights::WHITE_KINGSIDE,
            Square(56) => self.castling_rights &= !CastlingRights::BLACK_QUEENSIDE,
            Square(63) => self.castling_rights &= !CastlingRights::BLACK_KINGSIDE,
            _ => {}
        }
    }
}

fn parse_piece_char(ch: char) -> Option<(Color, Piece)> {
    let color = if ch.is_ascii_uppercase() {
        Color::White
    } else {
        Color::Black
    };

    let piece = match ch.to_ascii_lowercase() {
        'p' => Piece::Pawn,
        'n' => Piece::Knight,
        'b' => Piece::Bishop,
        'r' => Piece::Rook,
        'q' => Piece::Queen,
        'k' => Piece::King,
        _ => return None,
    };

    Some((color, piece))
}

fn parse_castling_rights(field: &str) -> Result<u8, FenError> {
    if field == "-" {
        return Ok(CastlingRights::NONE);
    }

    let mut rights = CastlingRights::NONE;
    for ch in field.chars() {
        match ch {
            'K' => rights |= CastlingRights::WHITE_KINGSIDE,
            'Q' => rights |= CastlingRights::WHITE_QUEENSIDE,
            'k' => rights |= CastlingRights::BLACK_KINGSIDE,
            'q' => rights |= CastlingRights::BLACK_QUEENSIDE,
            _ => return Err(FenError::InvalidCastlingRights),
        }
    }

    Ok(rights)
}

fn parse_en_passant(field: &str) -> Result<Option<Square>, FenError> {
    if field == "-" {
        return Ok(None);
    }

    let bytes = field.as_bytes();
    if bytes.len() != 2 {
        return Err(FenError::InvalidEnPassant);
    }

    let file = bytes[0];
    let rank = bytes[1];

    if !(b'a'..=b'h').contains(&file) || !(b'1'..=b'8').contains(&rank) {
        return Err(FenError::InvalidEnPassant);
    }

    Ok(Some(Square::from_rf(rank - b'1', file - b'a')))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_starting_position_fen() {
        let board = Board::starting_position();

        assert_eq!(board.side_to_move(), Color::White);
        assert_eq!(
            board.castling_rights(),
            CastlingRights::WHITE_KINGSIDE
                | CastlingRights::WHITE_QUEENSIDE
                | CastlingRights::BLACK_KINGSIDE
                | CastlingRights::BLACK_QUEENSIDE
        );
        assert_eq!(board.en_passant(), None);
        assert_eq!(board.halfmove_clock(), 0);
        assert_eq!(board.fullmove_number(), 1);

        assert_eq!(
            board.piece_on(Square::from_rf(0, 0)),
            Some((Color::White, Piece::Rook))
        );
        assert_eq!(
            board.piece_on(Square::from_rf(0, 4)),
            Some((Color::White, Piece::King))
        );
        assert_eq!(
            board.piece_on(Square::from_rf(7, 4)),
            Some((Color::Black, Piece::King))
        );
        assert_eq!(board.piece_on(Square::from_rf(3, 4)), None);
        assert_eq!(board.piece_bb(Color::White, Piece::Pawn).count_ones(), 8);
        assert_eq!(board.piece_bb(Color::Black, Piece::Pawn).count_ones(), 8);
        assert_eq!(board.occupancy(Color::White).count_ones(), 16);
        assert_eq!(board.occupancy(Color::Black).count_ones(), 16);
        assert_eq!(board.all_occupancy().count_ones(), 32);
    }

    #[test]
    fn parses_side_to_move_and_en_passant_square() {
        let board =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap();

        assert_eq!(board.side_to_move(), Color::Black);
        assert_eq!(board.en_passant(), Some(Square::from_rf(2, 4)));
        assert_eq!(
            board.piece_on(Square::from_rf(3, 4)),
            Some((Color::White, Piece::Pawn))
        );

        board.pretty_print();
    }

    #[test]
    fn rejects_invalid_field_count() {
        let err =
            Board::from_fen("8/8/8/8/8/8/8/8 w - - 0").expect_err("expected invalid field count");

        assert_eq!(err, FenError::InvalidFieldCount);
    }

    #[test]
    fn rejects_invalid_piece_placement() {
        let err = Board::from_fen("9/8/8/8/8/8/8/8 w - - 0 1")
            .expect_err("expected invalid piece placement");

        assert_eq!(err, FenError::InvalidPiecePlacement);
    }

    #[test]
    fn put_remove_and_move_piece_update_bitboards() {
        let mut board = Board::new();
        let b1 = Square::from_rf(0, 1);
        let c3 = Square::from_rf(2, 2);

        board.put_piece(b1, Color::White, Piece::Knight);
        assert_eq!(board.piece_on(b1), Some((Color::White, Piece::Knight)));
        assert_eq!(board.occupancy(Color::White).count_ones(), 1);

        board.move_piece(b1, c3, Color::White, Piece::Knight);
        assert_eq!(board.piece_on(b1), None);
        assert_eq!(board.piece_on(c3), Some((Color::White, Piece::Knight)));

        board.remove_piece(c3, Color::White, Piece::Knight);
        assert_eq!(board.piece_on(c3), None);
        assert_eq!(board.occupancy(Color::White), 0);
    }

    #[test]
    fn make_move_handles_double_pawn_push() {
        let mut board = Board::starting_position();
        board.make_move(Move::new(
            Square::from_rf(1, 4),
            Square::from_rf(3, 4),
            MoveKind::DoublePawnPush,
        ));

        assert_eq!(board.piece_on(Square::from_rf(1, 4)), None);
        assert_eq!(
            board.piece_on(Square::from_rf(3, 4)),
            Some((Color::White, Piece::Pawn))
        );
        assert_eq!(board.en_passant(), Some(Square::from_rf(2, 4)));
        assert_eq!(board.side_to_move(), Color::Black);
        assert_eq!(board.halfmove_clock(), 0);
        assert_eq!(board.fullmove_number(), 1);
    }

    #[test]
    fn make_move_handles_en_passant() {
        let mut board = Board::from_fen("8/8/8/3pP3/8/8/8/8 w - d6 0 1").unwrap();
        board.make_move(Move::new(
            Square::from_rf(4, 4),
            Square::from_rf(5, 3),
            MoveKind::EnPassant,
        ));

        assert_eq!(board.piece_on(Square::from_rf(4, 4)), None);
        assert_eq!(board.piece_on(Square::from_rf(4, 3)), None);
        assert_eq!(
            board.piece_on(Square::from_rf(5, 3)),
            Some((Color::White, Piece::Pawn))
        );
        assert_eq!(board.en_passant(), None);
    }

    #[test]
    fn make_move_handles_promotion_capture() {
        let mut board = Board::from_fen("1r6/P7/8/8/8/8/8/8 w - - 0 1").unwrap();
        board.make_move(Move::new(
            Square::from_rf(6, 0),
            Square::from_rf(7, 1),
            MoveKind::PromotionCapture(Piece::Queen),
        ));

        assert_eq!(board.piece_on(Square::from_rf(6, 0)), None);
        assert_eq!(
            board.piece_on(Square::from_rf(7, 1)),
            Some((Color::White, Piece::Queen))
        );
        assert_eq!(board.halfmove_clock(), 0);
    }

    #[test]
    fn make_move_handles_castling_and_castling_rights() {
        let mut board = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        board.make_move(Move::new(
            Square::from_rf(0, 4),
            Square::from_rf(0, 6),
            MoveKind::CastleKingside,
        ));

        assert_eq!(board.piece_on(Square::from_rf(0, 4)), None);
        assert_eq!(
            board.piece_on(Square::from_rf(0, 6)),
            Some((Color::White, Piece::King))
        );
        assert_eq!(
            board.piece_on(Square::from_rf(0, 5)),
            Some((Color::White, Piece::Rook))
        );
        assert_eq!(board.castling_rights() & CastlingRights::WHITE_KINGSIDE, 0);
        assert_eq!(board.castling_rights() & CastlingRights::WHITE_QUEENSIDE, 0);
    }

    #[test]
    fn make_move_handles_normal_capture() {
        let mut board = Board::from_fen("8/8/8/3p4/4P3/8/8/8 w - - 0 1").unwrap();

        board.make_move(Move::new(
            Square::from_rf(3, 4),
            Square::from_rf(3, 3),
            MoveKind::Capture,
        ));

        assert_eq!(
            board.piece_on(Square::from_rf(3, 3)),
            Some((Color::White, Piece::Pawn))
        );
    }

    #[test]
    fn unmake_move_reverts_double_pawn_push() {
        let mut board = Board::starting_position();
        let original = board.clone();

        board.make_move(Move::new(
            Square::from_rf(1, 4),
            Square::from_rf(3, 4),
            MoveKind::DoublePawnPush,
        ));
        board.unmake_move();

        assert_eq!(board, original);
    }

    #[test]
    fn unmake_move_reverts_en_passant() {
        let mut board = Board::from_fen("8/8/8/3pP3/8/8/8/8 w - d6 0 1").unwrap();
        let original = board.clone();

        board.make_move(Move::new(
            Square::from_rf(4, 4),
            Square::from_rf(5, 3),
            MoveKind::EnPassant,
        ));
        board.unmake_move();

        assert_eq!(board, original);
    }

    #[test]
    fn unmake_move_reverts_promotion_capture() {
        let mut board = Board::from_fen("1r6/P7/8/8/8/8/8/8 w - - 0 1").unwrap();
        let original = board.clone();

        board.make_move(Move::new(
            Square::from_rf(6, 0),
            Square::from_rf(7, 1),
            MoveKind::PromotionCapture(Piece::Queen),
        ));
        board.unmake_move();

        assert_eq!(board, original);
    }

    #[test]
    fn unmake_move_reverts_castling() {
        let mut board = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let original = board.clone();

        board.make_move(Move::new(
            Square::from_rf(0, 4),
            Square::from_rf(0, 6),
            MoveKind::CastleKingside,
        ));
        board.unmake_move();

        assert_eq!(board, original);
    }
}
