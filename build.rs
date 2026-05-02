use std::env;
use std::fs;
use std::path::PathBuf;

type Bitboard = u64;

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

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let generated = out_dir.join("generated_attacks.rs");
    fs::write(generated, generate()).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}

fn generate() -> String {
    let mut out = String::new();
    out.push_str("pub(crate) const ROOK_MAGICS: [u64; 64] = ");
    push_array(&mut out, &ROOK_MAGICS);
    out.push_str("pub(crate) const BISHOP_MAGICS: [u64; 64] = ");
    push_array(&mut out, &BISHOP_MAGICS);
    out.push_str("pub(crate) const ROOK_MASKS: [u64; 64] = ");
    push_array(&mut out, &generate_masks(true));
    out.push_str("pub(crate) const BISHOP_MASKS: [u64; 64] = ");
    push_array(&mut out, &generate_masks(false));
    out.push_str("pub(crate) const ROOK_TABLE: [[u64; 4096]; 64] = ");
    push_2d_array(&mut out, &generate_tables(true));
    out.push_str("pub(crate) const BISHOP_TABLE: [[u64; 512]; 64] = ");
    push_2d_array(&mut out, &generate_tables(false));
    out
}

fn generate_masks(rook: bool) -> [u64; 64] {
    std::array::from_fn(|sq| {
        let r = (sq / 8) as i32;
        let f = (sq % 8) as i32;
        let mut mask = 0u64;
        if rook {
            for rr in (r + 1)..7 {
                mask |= 1u64 << (rr * 8 + f);
            }
            for rr in 1..r {
                mask |= 1u64 << (rr * 8 + f);
            }
            for ff in (f + 1)..7 {
                mask |= 1u64 << (r * 8 + ff);
            }
            for ff in 1..f {
                mask |= 1u64 << (r * 8 + ff);
            }
        } else {
            let mut rr = r + 1;
            let mut ff = f + 1;
            while rr < 7 && ff < 7 {
                mask |= 1u64 << (rr * 8 + ff);
                rr += 1;
                ff += 1;
            }
            let mut rr = r + 1;
            let mut ff = f - 1;
            while rr < 7 && ff > 0 {
                mask |= 1u64 << (rr * 8 + ff);
                rr += 1;
                ff -= 1;
            }
            let mut rr = r - 1;
            let mut ff = f + 1;
            while rr > 0 && ff < 7 {
                mask |= 1u64 << (rr * 8 + ff);
                rr -= 1;
                ff += 1;
            }
            let mut rr = r - 1;
            let mut ff = f - 1;
            while rr > 0 && ff > 0 {
                mask |= 1u64 << (rr * 8 + ff);
                rr -= 1;
                ff -= 1;
            }
        }
        mask
    })
}

fn generate_tables(rook: bool) -> Vec<Vec<u64>> {
    let masks = if rook {
        generate_masks(true)
    } else {
        generate_masks(false)
    };
    let magics = if rook { ROOK_MAGICS } else { BISHOP_MAGICS };
    let size = if rook { 4096 } else { 512 };
    let mut tables = vec![vec![0u64; size]; 64];
    for sq in 0..64 {
        let mask = masks[sq];
        let shift = 64 - mask.count_ones();
        let mut subset = 0u64;
        loop {
            let idx = (((subset & mask).wrapping_mul(magics[sq])) >> shift) as usize;
            tables[sq][idx] = if rook {
                ray(sq, subset, true)
            } else {
                ray(sq, subset, false)
            };
            subset = (subset.wrapping_sub(mask)) & mask;
            if subset == 0 {
                let idx = (((subset & mask).wrapping_mul(magics[sq])) >> shift) as usize;
                tables[sq][idx] = if rook {
                    ray(sq, subset, true)
                } else {
                    ray(sq, subset, false)
                };
                break;
            }
        }
    }
    tables
}

fn ray(sq: usize, occupancy: u64, rook: bool) -> u64 {
    let r = (sq / 8) as i32;
    let f = (sq % 8) as i32;
    let dirs = if rook {
        &[(1, 0), (-1, 0), (0, 1), (0, -1)][..]
    } else {
        &[(1, 1), (1, -1), (-1, 1), (-1, -1)][..]
    };
    let mut attacks = 0u64;
    for &(dr, df) in dirs {
        let mut rr = r + dr;
        let mut ff = f + df;
        while (0..8).contains(&rr) && (0..8).contains(&ff) {
            let bb = 1u64 << (rr * 8 + ff);
            attacks |= bb;
            if occupancy & bb != 0 {
                break;
            }
            rr += dr;
            ff += df;
        }
    }
    attacks
}

fn push_array<T: std::fmt::LowerHex + Copy, const N: usize>(out: &mut String, arr: &[T; N]) {
    out.push('[');
    for (i, v) in arr.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("0x{:x}", v));
    }
    out.push_str("];\n");
}

fn push_2d_array(out: &mut String, arr: &Vec<Vec<u64>>) {
    out.push('[');
    for (i, row) in arr.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push('[');
        for (j, v) in row.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("0x{:x}", v));
        }
        out.push(']');
    }
    out.push_str("];\n");
}
