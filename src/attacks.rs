use std::sync::OnceLock;

use crate::bitboard::Bitboard;
use crate::types::{Color, Square};

const ROOK_MAGICS: [Bitboard; 64] = [
    0x0080001020400080, 0x0040001000200040, 0x0080081000200080, 0x0080040800100080,
    0x0080020400080080, 0x0080010200040080, 0x0080008001000200, 0x0080002040800100,
    0x0000800020400080, 0x0000400020005000, 0x0000801000200080, 0x0000800800100080,
    0x0000800400080080, 0x0000800200040080, 0x0000800100020080, 0x0000800040800100,
    0x0000208000400080, 0x0000404000201000, 0x0000808010002000, 0x0000808008001000,
    0x0000808004000800, 0x0000808002000400, 0x0000010100020004, 0x0000020000408104,
    0x0000208080004000, 0x0000200040005000, 0x0000100080200080, 0x0000080080100080,
    0x0000040080080080, 0x0000020080040080, 0x0000010080800200, 0x0000800080004100,
    0x0000204000800080, 0x0000200040401000, 0x0000100080802000, 0x0000080080801000,
    0x0000040080800800, 0x0000020080800400, 0x0000020001010004, 0x0000800040800100,
    0x0000204000808000, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
    0x0000040008008080, 0x0000020004008080, 0x0000010002008080, 0x0000004081020004,
    0x0000204000800080, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
    0x0000040008008080, 0x0000020004008080, 0x0000800100020080, 0x0000800041000080,
    0x00FFFCDDFCED714A, 0x007FFCDDFCED714A, 0x003FFFCDFFD88096, 0x0000040810002101,
    0x0001000204080011, 0x0001000204000801, 0x0001000082000401, 0x0001FFFAABFAD1A2,
];

const BISHOP_MAGICS: [Bitboard; 64] = [
    0x0002020202020200, 0x0002020202020000, 0x0004010202000000, 0x0004040080000000,
    0x0001104000000000, 0x0000821040000000, 0x0000410410400000, 0x0000104104104000,
    0x0000040404040400, 0x0000020202020200, 0x0000040102020000, 0x0000040400800000,
    0x0000011040000000, 0x0000008210400000, 0x0000004104104000, 0x0000002082082000,
    0x0004000808080800, 0x0002000404040400, 0x0001000202020200, 0x0000800802004000,
    0x0000800400A00000, 0x0000200100884000, 0x0000400082082000, 0x0000200041041000,
    0x0002080010101000, 0x0001040008080800, 0x0000208004010400, 0x0000404004010200,
    0x0000840000802000, 0x0000404002011000, 0x0000808001041000, 0x0000404000820800,
    0x0001041000202000, 0x0000820800101000, 0x0000104400080800, 0x0000020080080080,
    0x0000404040040100, 0x0000808100020100, 0x0001010100020800, 0x0000808080010400,
    0x0000820820004000, 0x0000410410002000, 0x0000082088001000, 0x0000002011000800,
    0x0000080100400400, 0x0001010101000200, 0x0002020202000400, 0x0001010101000200,
    0x0000410410400000, 0x0000208208200000, 0x0000002084100000, 0x0000000020880000,
    0x0000001002020000, 0x0000040408020000, 0x0004040404040000, 0x0002020202020000,
    0x0000104104104000, 0x0000002082082000, 0x0000000020841000, 0x0000000000208800,
    0x0000000010020200, 0x0000000404080200, 0x0000040404040400, 0x0002020202020200,
];

#[derive(Debug)]
pub struct AttackTable {
    pub knight: [Bitboard; 64],
    pub king: [Bitboard; 64],
    pub pawn: [[Bitboard; 64]; 2],
    rook: SlidingAttacks,
    bishop: SlidingAttacks,
}

impl AttackTable {
    pub fn init() -> Self {
        let mut table = AttackTable {
            knight: [0; 64],
            king: [0; 64],
            pawn: [[0; 64]; 2],
            rook: SlidingAttacks::build(AttackKind::Rook),
            bishop: SlidingAttacks::build(AttackKind::Bishop),
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
    attack_table().rook.attacks(square, occupancy)
}

pub fn bishop_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    attack_table().bishop.attacks(square, occupancy)
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

fn ray_attacks(square: Square, occupancy: Bitboard, directions: &[(i8, i8)]) -> Bitboard {
    let mut attacks = 0;
    let start_rank = square.rank() as i8;
    let start_file = square.file() as i8;

    for &(dr, df) in directions {
        let mut rank = start_rank + dr;
        let mut file = start_file + df;

        while (0..8).contains(&rank) && (0..8).contains(&file) {
            let target = Square::from_rf(rank as u8, file as u8);
            attacks |= target.to_bb();

            if target.to_bb() & occupancy != 0 {
                break;
            }

            rank += dr;
            file += df;
        }
    }

    attacks
}

#[derive(Debug)]
struct SlidingAttacks {
    masks: [Bitboard; 64],
    magics: [Bitboard; 64],
    shifts: [u32; 64],
    tables: Vec<Vec<Bitboard>>,
}

#[derive(Debug, Clone, Copy)]
enum AttackKind {
    Rook,
    Bishop,
}

impl SlidingAttacks {
    fn build(kind: AttackKind) -> Self {
        let mut masks = [0; 64];
        let mut magics = [0; 64];
        let mut shifts = [0; 64];
        let mut tables = Vec::with_capacity(64);

        for sq in 0..64 {
            let square = Square::from_index(sq as u8);
            let mask = sliding_mask(square, kind);
            let relevant_bits = mask.count_ones() as usize;
            let table_size = 1usize << relevant_bits;
            let mut table = vec![0; table_size];
            let magic = match kind {
                AttackKind::Rook => ROOK_MAGICS[sq],
                AttackKind::Bishop => BISHOP_MAGICS[sq],
            };
            let shift = 64 - relevant_bits as u32;

            for occ in enumerate_subsets(mask) {
                let idx = magic_index(occ, mask, magic, shift);
                table[idx] = match kind {
                    AttackKind::Rook => ray_attacks(square, occ, &[(1, 0), (-1, 0), (0, 1), (0, -1)]),
                    AttackKind::Bishop => ray_attacks(square, occ, &[(1, 1), (1, -1), (-1, 1), (-1, -1)]),
                };
            }

            masks[sq] = mask;
            magics[sq] = magic;
            shifts[sq] = shift;
            tables.push(table);
        }

        Self {
            masks,
            magics,
            shifts,
            tables,
        }
    }

    fn attacks(&self, square: Square, occupancy: Bitboard) -> Bitboard {
        let idx = square.index();
        let key = magic_index(occupancy, self.masks[idx], self.magics[idx], self.shifts[idx]);
        self.tables[idx][key]
    }
}

fn sliding_mask(square: Square, kind: AttackKind) -> Bitboard {
    match kind {
        AttackKind::Rook => {
            let mut mask = ray_attacks(square, 0, &[(1, 0), (-1, 0), (0, 1), (0, -1)]);
            let rank = square.rank();
            let file = square.file();

            for r in [0, 7] {
                if r != rank {
                    for f in 0..8 {
                        mask &= !Square::from_rf(r, f).to_bb();
                    }
                }
            }
            for f in [0, 7] {
                if f != file {
                    for r in 0..8 {
                        mask &= !Square::from_rf(r, f).to_bb();
                    }
                }
            }

            mask
        }
        AttackKind::Bishop => {
            let mut mask = ray_attacks(square, 0, &[(1, 1), (1, -1), (-1, 1), (-1, -1)]);
            for r in [0, 7] {
                for f in 0..8 {
                    mask &= !Square::from_rf(r, f).to_bb();
                }
            }
            for f in [0, 7] {
                for r in 0..8 {
                    mask &= !Square::from_rf(r, f).to_bb();
                }
            }
            mask
        }
    }
}

fn enumerate_subsets(mask: Bitboard) -> Vec<Bitboard> {
    let mut bits = Vec::new();
    let mut m = mask;
    while m != 0 {
        let sq = m.trailing_zeros() as u8;
        bits.push(1u64 << sq);
        m &= m - 1;
    }

    let mut subsets = vec![0];
    for bit in bits {
        let mut next = subsets.clone();
        for value in &mut next {
            *value |= bit;
        }
        subsets.extend(next);
    }
    subsets
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
