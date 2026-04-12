use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn opposite(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Piece {
    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn score(self) -> i32 {
        match self {
            Piece::Pawn => 100,
            Piece::Knight => 320,
            Piece::Bishop => 330,
            Piece::Rook => 500,
            Piece::Queen => 900,
            Piece::King => 20000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Square(pub u8);

impl Square {
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub const fn rank(self) -> u8 {
        self.0 / 8
    }
    pub const fn file(self) -> u8 {
        self.0 % 8
    }

    pub const fn from_index(index: u8) -> Self {
        Square(index)
    }

    pub const fn from_rf(rank: u8, file: u8) -> Square {
        Square(rank * 8 + file)
    }

    pub const fn to_bb(self) -> u64 {
        1u64 << self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    Quiet,
    DoublePawnPush,
    Capture,
    EnPassant,
    CastleKingside,
    CastleQueenside,
    Promotion(Piece),
    PromotionCapture(Piece),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub kind: MoveKind,
}

impl Move {
    pub const fn new(from: Square, to: Square, kind: MoveKind) -> Self {
        Move { from, to, kind }
    }

    pub fn is_capture(self) -> bool {
        matches!(
            self.kind,
            MoveKind::Capture | MoveKind::EnPassant | MoveKind::PromotionCapture(_)
        )
    }

    pub fn is_promotion(self) -> bool {
        matches!(
            self.kind,
            MoveKind::Promotion(_) | MoveKind::PromotionCapture(_)
        )
    }

    pub fn is_castle(self) -> bool {
        matches!(
            self.kind,
            MoveKind::CastleKingside | MoveKind::CastleQueenside
        )
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();

        // from square
        let from_file = (b'a' + self.from.file() as u8) as char;
        let from_rank = (b'1' + self.from.rank() as u8) as char;

        // to square
        let to_file = (b'a' + self.to.file() as u8) as char;
        let to_rank = (b'1' + self.to.rank() as u8) as char;

        s.push(from_file);
        s.push(from_rank);
        s.push(to_file);
        s.push(to_rank);

        // promotions
        match self.kind {
            MoveKind::Promotion(piece) | MoveKind::PromotionCapture(piece) => {
                s.push(match piece {
                    Piece::Knight => 'n',
                    Piece::Bishop => 'b',
                    Piece::Rook => 'r',
                    Piece::Queen => 'q',
                    _ => unreachable!(),
                });
            }
            _ => {}
        }

        write!(f, "{}", s)
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

pub struct CastlingRights;

impl CastlingRights {
    pub const NONE: u8 = 0;
    pub const WHITE_KINGSIDE: u8 = 1 << 0;
    pub const WHITE_QUEENSIDE: u8 = 1 << 1;
    pub const BLACK_KINGSIDE: u8 = 1 << 2;
    pub const BLACK_QUEENSIDE: u8 = 1 << 3;
}
