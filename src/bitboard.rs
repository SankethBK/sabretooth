use crate::types::Square;

pub type Bitboard = u64;

fn bit(square: Square) -> Bitboard {
    1u64 << square.0
}

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

pub fn pretty_print(bb: Bitboard) {
    for rank in (0..8).rev() {
        for file in 0..8 {
            let sq = rank * 8 + file;
            if bb & (1u64 << sq) != 0 {
                print!("1 ");
            } else {
                print!(". ");
            }
        }
        println!("  {}", rank + 1);
    }
    println!("a b c d e f g h");
    println!("0x{:016X}\n", bb);
}
