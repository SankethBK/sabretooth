use std::sync::OnceLock;

use crate::bitboard::Bitboard;
use crate::types::{Color, Square};
include!(concat!(env!("OUT_DIR"), "/generated_attacks.rs"));

#[derive(Debug)]
pub struct AttackTable {
    pub knight: [Bitboard; 64],
    pub king: [Bitboard; 64],
    pub pawn: [[Bitboard; 64]; 2],
    rook_masks: &'static [Bitboard; 64],
    bishop_masks: &'static [Bitboard; 64],
    rook_tables: &'static [[Bitboard; 4096]; 64],
    bishop_tables: &'static [[Bitboard; 512]; 64],
}

impl AttackTable {
    pub fn init() -> Self {
        let mut table = AttackTable {
            knight: [0; 64],
            king: [0; 64],
            pawn: [[0; 64]; 2],
            rook_masks: &ROOK_MASKS,
            bishop_masks: &BISHOP_MASKS,
            rook_tables: &ROOK_TABLE,
            bishop_tables: &BISHOP_TABLE,
        };

        for sq in 0..64 {
            let square = Square::from_index(sq);
            table.knight[sq as usize] = compute_knight_attacks(square);
            table.king[sq as usize] = compute_king_attacks(square);
            table.pawn[Color::White.index()][sq as usize] =
                compute_pawn_attacks(Color::White, square);
            table.pawn[Color::Black.index()][sq as usize] =
                compute_pawn_attacks(Color::Black, square);
        }

        table
    }
}

static ATTACK_TABLE: OnceLock<AttackTable> = OnceLock::new();

pub fn attack_table() -> &'static AttackTable {
    ATTACK_TABLE.get_or_init(AttackTable::init)
}

pub fn compute_knight_attacks(square: Square) -> Bitboard {
    let rank = square.rank() as i8;
    let file = square.file() as i8;
    let offsets = [
        (2, 1),
        (2, -1),
        (-2, 1),
        (-2, -1),
        (1, 2),
        (1, -2),
        (-1, 2),
        (-1, -2),
    ];

    compute_attacks_from_offsets(rank, file, &offsets)
}

pub fn compute_king_attacks(square: Square) -> Bitboard {
    let rank = square.rank() as i8;
    let file = square.file() as i8;
    let offsets = [
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
        (0, -1),
        (1, -1),
    ];

    compute_attacks_from_offsets(rank, file, &offsets)
}

pub fn compute_pawn_attacks(color: Color, square: Square) -> Bitboard {
    let rank = square.rank() as i8;
    let file = square.file() as i8;
    let offsets = match color {
        Color::White => [(1, -1), (1, 1)],
        Color::Black => [(-1, -1), (-1, 1)],
    };

    compute_attacks_from_offsets(rank, file, &offsets)
}

pub fn rook_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    let table = attack_table();
    let idx = magic_index(occupancy, table.rook_masks[square.index()], ROOK_MAGICS[square.index()], 64 - table.rook_masks[square.index()].count_ones());
    table.rook_tables[square.index()][idx]
}

pub fn bishop_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    let table = attack_table();
    let idx = magic_index(occupancy, table.bishop_masks[square.index()], BISHOP_MAGICS[square.index()], 64 - table.bishop_masks[square.index()].count_ones());
    table.bishop_tables[square.index()][idx]
}

pub fn queen_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    rook_attacks(square, occupancy) | bishop_attacks(square, occupancy)
}

fn compute_attacks_from_offsets(rank: i8, file: i8, offsets: &[(i8, i8)]) -> Bitboard {
    let mut attacks = 0;

    for &(dr, df) in offsets {
        let next_rank = rank + dr;
        let next_file = file + df;

        if (0..8).contains(&next_rank) && (0..8).contains(&next_file) {
            attacks |= Square::from_rf(next_rank as u8, next_file as u8).to_bb();
        }
    }

    attacks
}

fn magic_index(occupancy: Bitboard, mask: Bitboard, magic: Bitboard, shift: u32) -> usize {
    (((occupancy & mask).wrapping_mul(magic)) >> shift) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knight_attacks_from_center() {
        let attacks = attack_table().knight[Square::from_rf(3, 3).index()];

        assert_eq!(attacks.count_ones(), 8);
        assert_ne!(attacks & Square::from_rf(5, 4).to_bb(), 0);
        assert_ne!(attacks & Square::from_rf(4, 5).to_bb(), 0);
        assert_ne!(attacks & Square::from_rf(1, 2).to_bb(), 0);
        assert_eq!(attacks & Square::from_rf(3, 3).to_bb(), 0);
    }

    #[test]
    fn knight_attacks_from_corner() {
        let attacks = attack_table().knight[Square::from_rf(0, 0).index()];

        assert_eq!(attacks.count_ones(), 2);
        assert_ne!(attacks & Square::from_rf(1, 2).to_bb(), 0);
        assert_ne!(attacks & Square::from_rf(2, 1).to_bb(), 0);
    }

    #[test]
    fn king_attacks_from_corner() {
        let attacks = attack_table().king[Square::from_rf(0, 0).index()];

        assert_eq!(attacks.count_ones(), 3);
        assert_ne!(attacks & Square::from_rf(0, 1).to_bb(), 0);
        assert_ne!(attacks & Square::from_rf(1, 0).to_bb(), 0);
        assert_ne!(attacks & Square::from_rf(1, 1).to_bb(), 0);
    }

    #[test]
    fn pawn_attacks_depend_on_color() {
        let white = attack_table().pawn[Color::White.index()][Square::from_rf(3, 3).index()];
        let black = attack_table().pawn[Color::Black.index()][Square::from_rf(3, 3).index()];

        assert_eq!(white.count_ones(), 2);
        assert_eq!(black.count_ones(), 2);
        assert_ne!(white & Square::from_rf(4, 2).to_bb(), 0);
        assert_ne!(white & Square::from_rf(4, 4).to_bb(), 0);
        assert_ne!(black & Square::from_rf(2, 2).to_bb(), 0);
        assert_ne!(black & Square::from_rf(2, 4).to_bb(), 0);
    }

    #[test]
    fn rook_attacks_from_center_on_empty_board() {
        let attacks = rook_attacks(Square::from_rf(3, 3), 0);

        assert_eq!(attacks.count_ones(), 14);
        assert_ne!(attacks & Square::from_rf(3, 0).to_bb(), 0);
        assert_ne!(attacks & Square::from_rf(3, 7).to_bb(), 0);
        assert_ne!(attacks & Square::from_rf(0, 3).to_bb(), 0);
        assert_ne!(attacks & Square::from_rf(7, 3).to_bb(), 0);
    }

    #[test]
    fn rook_attacks_stop_after_first_blocker() {
        let occupancy = Square::from_rf(3, 5).to_bb() | Square::from_rf(1, 3).to_bb();
        let attacks = rook_attacks(Square::from_rf(3, 3), occupancy);

        assert_ne!(attacks & Square::from_rf(3, 5).to_bb(), 0);
        assert_eq!(attacks & Square::from_rf(3, 6).to_bb(), 0);
        assert_ne!(attacks & Square::from_rf(1, 3).to_bb(), 0);
        assert_eq!(attacks & Square::from_rf(0, 3).to_bb(), 0);
    }

    #[test]
    fn bishop_attacks_from_center_on_empty_board() {
        let attacks = bishop_attacks(Square::from_rf(3, 3), 0);

        assert_eq!(attacks.count_ones(), 13);
        assert_ne!(attacks & Square::from_rf(0, 0).to_bb(), 0);
        assert_ne!(attacks & Square::from_rf(6, 6).to_bb(), 0);
        assert_ne!(attacks & Square::from_rf(0, 6).to_bb(), 0);
        assert_ne!(attacks & Square::from_rf(6, 0).to_bb(), 0);
    }

    #[test]
    fn bishop_attacks_stop_after_first_blocker() {
        let occupancy = Square::from_rf(5, 5).to_bb() | Square::from_rf(1, 5).to_bb();
        let attacks = bishop_attacks(Square::from_rf(3, 3), occupancy);

        assert_ne!(attacks & Square::from_rf(5, 5).to_bb(), 0);
        assert_eq!(attacks & Square::from_rf(6, 6).to_bb(), 0);
        assert_ne!(attacks & Square::from_rf(1, 5).to_bb(), 0);
        assert_eq!(attacks & Square::from_rf(0, 6).to_bb(), 0);
    }

    #[test]
    fn queen_attacks_are_rook_and_bishop_union() {
        let occupancy = Square::from_rf(5, 3).to_bb() | Square::from_rf(5, 5).to_bb();
        let rook = rook_attacks(Square::from_rf(3, 3), occupancy);
        let bishop = bishop_attacks(Square::from_rf(3, 3), occupancy);
        let queen = queen_attacks(Square::from_rf(3, 3), occupancy);

        assert_eq!(queen, rook | bishop);
    }
}
