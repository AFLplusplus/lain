use crate::rand::Rng;
use crate::traits::*;

static DANGEROUS_NUMBERS_U8: &[u8] = &[
    u8::MIN,             // 0x00
    u8::MAX,             // 0xff
    i8::MAX as u8,       // 0x7f
    (i8::MAX as u8) + 1, // 0x80
];

static DANGEROUS_NUMBERS_U16: &[u16] = &[
    // big-endian variants
    u16::MIN,              // 0x0000
    u16::MAX,              // 0xffff
    i16::MAX as u16,       // 0x7fff
    (i16::MAX as u16) + 1, // 0x8000
    // little-endian variants
    0xff7f,
    0x0080,
];

static DANGEROUS_NUMBERS_U32: &[u32] = &[
    // big-endian variants
    u32::MIN,
    u32::MAX,
    i32::MAX as u32,
    (i32::MAX as u32) + 1,
    // little-endian variants
    0xffff_ff7f,
    0x0000_0080,
];

static DANGEROUS_NUMBERS_U64: &[u64] = &[
    // big-endian variants
    u64::MIN,
    u64::MAX,
    i64::MAX as u64,
    (i64::MAX as u64) + 1,
    // little-endian variants
    0xffff_ffff_ffff_ff7f,
    0x0000_0000_0000_0080,
];

static DANGEROUS_NUMBERS_F32: &[f32] = &[
    f32::INFINITY,
    f32::MAX,
    f32::MIN,
    f32::MIN_POSITIVE,
    f32::NAN,
    f32::NEG_INFINITY,
];

static DANGEROUS_NUMBERS_F64: &[f64] = &[
    f64::INFINITY,
    f64::MAX,
    f64::MIN,
    f64::MIN_POSITIVE,
    f64::NAN,
    f64::NEG_INFINITY,
];

macro_rules! dangerous_number {
    ( $ty:ident, $nums:ident ) => {
        impl DangerousNumber<$ty> for $ty {
            fn select_dangerous_number<R: Rng>(rng: &mut R) -> $ty {
                return $nums[rng.random_range(0..$nums.len())] as $ty;
            }

            fn dangerous_number_at_index(idx: usize) -> $ty {
                $nums[idx] as $ty
            }

            fn dangerous_numbers_len() -> usize {
                $nums.len()
            }
        }
    };
}

dangerous_number!(u8, DANGEROUS_NUMBERS_U8);
dangerous_number!(i8, DANGEROUS_NUMBERS_U8);
dangerous_number!(u16, DANGEROUS_NUMBERS_U16);
dangerous_number!(i16, DANGEROUS_NUMBERS_U16);
dangerous_number!(u32, DANGEROUS_NUMBERS_U32);
dangerous_number!(i32, DANGEROUS_NUMBERS_U32);
dangerous_number!(u64, DANGEROUS_NUMBERS_U64);
dangerous_number!(i64, DANGEROUS_NUMBERS_U64);
dangerous_number!(f32, DANGEROUS_NUMBERS_F32);
dangerous_number!(f64, DANGEROUS_NUMBERS_F64);
