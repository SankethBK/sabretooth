use crate::types::Square;

pub type Bitboard = u64;

pub fn lsb(bb: Bitboard) -> Square {
    Square::from_index(bb.trailing_zeros() as u8) //  same as __builtin_ctzll in C++
}

pub fn popcount(bb: Bitboard) -> u32 {
    bb.count_ones() //  same as __builtin_popcountll in C++
}

pub fn pop_lsb(bb: &mut Bitboard) -> Square {
    let sq = lsb(*bb);
    *bb &= *bb - 1;
    sq
}

pub fn pretty_print_bitboard(bb: Bitboard) -> String {
    let mut out = String::new();

    for rank in (0..8).rev() {
        for file in 0..8 {
            let sq = rank * 8 + file;
            if bb & (1u64 << sq) != 0 {
                out.push_str("1 ");
            } else {
                out.push_str(". ");
            }
        }
        out.push_str(&format!("  {}\n", rank + 1));
    }

    out.push_str("a b c d e f g h\n");
    out.push_str(&format!("0x{:016X}\n", bb));

    out
}
