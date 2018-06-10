/*
 *  demixer - file compressor aimed at high compression ratios
 *  Copyright (C) 2018  Piotr Tarsa ( https://github.com/tarsa )
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
pub mod types;

use std::fmt::Debug;

use DO_CHECKS;
use fixed_point::types::Log2D;
use lut::log2::Log2Lut;

pub trait FixedPoint where Self: Sized {
    type Raw: FloatConversions + Copy + Debug;

    fn raw(&self) -> Self::Raw;
    fn new_unchecked(raw: Self::Raw) -> Self;
    fn new(raw: Self::Raw, fractional_bits: u8) -> Self {
        if DO_CHECKS {
            assert_eq!(fractional_bits, Self::FRACTIONAL_BITS,
                       "fractional bits numbers don't match");
        }
        let result = Self::new_unchecked(raw);
        if DO_CHECKS { assert!(result.within_bounds(), "{:?}", raw); }
        result
    }
    fn within_bounds(&self) -> bool { true }

    const FRACTIONAL_BITS: u8;

    fn from_f32(input: f32) -> Self { Self::from_f64(input as f64) }
    fn as_f32(&self) -> f32 { self.as_f64() as f32 }

    fn from_f64(input: f64) -> Self {
        Self::new(
            Self::Raw::from_f64(input * (1u64 << Self::FRACTIONAL_BITS) as f64),
            Self::FRACTIONAL_BITS)
    }

    fn as_f64(&self) -> f64 {
        self.raw().into_f64() / (1u64 << Self::FRACTIONAL_BITS) as f64
    }
}

pub mod fix_i16 {
    use DO_CHECKS;

    pub fn scaled_down(raw: i16, shift: u8) -> i16 {
        if DO_CHECKS { assert_ne!(shift, 0); }
        raw.saturating_add(raw.signum() << (shift - 1))
            .max(i16::min_value() + 1) / (1 << shift)
    }
}

pub mod fix_u16 {
    use DO_CHECKS;

    pub fn scaled_down(raw: u16, shift: u8) -> u16 {
        if DO_CHECKS { assert_ne!(shift, 0); }
        raw.saturating_add(1 << (shift - 1)) >> shift
    }
}

pub trait FixI32: FixedPoint<Raw=i32> {
    const TOTAL_BITS: u8 = 32;
    const INTEGRAL_BITS: u8 = Self::TOTAL_BITS - Self::FRACTIONAL_BITS;

    fn ulp() -> Self {
        Self::new_unchecked(1)
    }

    fn trunc(&self) -> Self::Raw {
        self.raw() >> Self::FRACTIONAL_BITS
    }

    fn to_fix_i32<R: FixI32>(&self) -> R {
        let raw = self.raw();
        if Self::FRACTIONAL_BITS > R::FRACTIONAL_BITS {
            let bits_diff = Self::FRACTIONAL_BITS - R::FRACTIONAL_BITS;
            R::new(fix_i32::scaled_down(raw, bits_diff), R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - Self::FRACTIONAL_BITS;
            if DO_CHECKS { assert_eq!(raw << bits_diff >> bits_diff, raw); }
            R::new(raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    fn to_fix_i64<R: FixI64>(&self) -> R {
        let raw = self.raw() as i64;
        if Self::FRACTIONAL_BITS > R::FRACTIONAL_BITS {
            let bits_diff = Self::FRACTIONAL_BITS - R::FRACTIONAL_BITS;
            R::new(fix_i64::scaled_down(raw, bits_diff), R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - Self::FRACTIONAL_BITS;
            if DO_CHECKS { assert_eq!(raw << bits_diff >> bits_diff, raw); }
            R::new(raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    fn add(&self, other: &Self) -> Self {
        let add_raw =
            if DO_CHECKS {
                self.raw().checked_add(other.raw())
                    .expect("numeric range exceeded")
            } else {
                self.raw() + other.raw()
            };
        Self::new(add_raw, Self::FRACTIONAL_BITS)
    }

    fn sub(&self, other: &Self) -> Self {
        let sub_raw =
            if DO_CHECKS {
                self.raw().checked_sub(other.raw())
                    .expect("numeric range exceeded")
            } else {
                self.raw() - other.raw()
            };
        Self::new(sub_raw, Self::FRACTIONAL_BITS)
    }

    fn neg(&self) -> Self {
        Self::new(-self.raw(), Self::FRACTIONAL_BITS)
    }
}

pub mod fix_i32 {
    use DO_CHECKS;
    use super::{FixI32, FixI64, fix_i64};

    pub fn scaled_down(raw: i32, shift: u8) -> i32 {
        if DO_CHECKS { assert_ne!(shift, 0); }
        raw.saturating_add(raw.signum() << (shift - 1))
            .max(i32::min_value() + 1) / (1 << shift)
    }

    pub fn mul<A: FixI32, B: FixI32, R: FixI32>(a: &A, b: &B) -> R {
        let mul_raw = a.raw() as i64 * b.raw() as i64;
        let total_fract_bits = A::FRACTIONAL_BITS + B::FRACTIONAL_BITS;
        if total_fract_bits > R::FRACTIONAL_BITS {
            let bits_diff = total_fract_bits - R::FRACTIONAL_BITS;
            let result_wide = fix_i64::scaled_down(mul_raw, bits_diff);
            if DO_CHECKS {
                assert_eq!((result_wide as i32) as i64, result_wide);
            }
            R::new(result_wide as i32, R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - total_fract_bits;
            let result_wide = mul_raw << bits_diff;
            if DO_CHECKS {
                assert_eq!((result_wide as i32) as i64, result_wide);
                assert_eq!(result_wide >> bits_diff, mul_raw);
            }
            R::new(result_wide as i32, R::FRACTIONAL_BITS)
        }
    }

    pub fn mul_wide<A: FixI32, B: FixI32, R: FixI64>(a: &A, b: &B) -> R {
        let mul_raw = a.raw() as i64 * b.raw() as i64;
        let total_fract_bits = A::FRACTIONAL_BITS + B::FRACTIONAL_BITS;
        if total_fract_bits > R::FRACTIONAL_BITS {
            let bits_diff = total_fract_bits - R::FRACTIONAL_BITS;
            R::new(fix_i64::scaled_down(mul_raw, bits_diff), R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - total_fract_bits;
            if DO_CHECKS {
                assert_eq!(mul_raw << bits_diff >> bits_diff, mul_raw);
            }
            R::new(mul_raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }
}

pub trait FixU32: FixedPoint<Raw=u32> {
    const TOTAL_BITS: u8 = 32;
    const INTEGRAL_BITS: u8 = Self::TOTAL_BITS - Self::FRACTIONAL_BITS;

    fn ulp() -> Self {
        Self::new_unchecked(1)
    }

    fn trunc(&self) -> Self::Raw {
        self.raw() >> Self::FRACTIONAL_BITS
    }

    fn to_fix_u32<R: FixU32>(&self) -> R {
        let raw = self.raw();
        if Self::FRACTIONAL_BITS > R::FRACTIONAL_BITS {
            let bits_diff = Self::FRACTIONAL_BITS - R::FRACTIONAL_BITS;
            R::new(fix_u32::scaled_down(raw, bits_diff), R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - Self::FRACTIONAL_BITS;
            if DO_CHECKS { assert_eq!(raw << bits_diff >> bits_diff, raw); }
            R::new(raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    fn to_fix_u64<R: FixU64>(&self) -> R {
        let raw = self.raw() as u64;
        if Self::FRACTIONAL_BITS > R::FRACTIONAL_BITS {
            let bits_diff = Self::FRACTIONAL_BITS - R::FRACTIONAL_BITS;
            R::new(fix_u64::scaled_down(raw, bits_diff), R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - Self::FRACTIONAL_BITS;
            if DO_CHECKS { assert_eq!(raw << bits_diff >> bits_diff, raw); }
            R::new(raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    fn add(&self, other: &Self) -> Self {
        let add_raw =
            if DO_CHECKS {
                self.raw().checked_add(other.raw())
                    .expect("numeric range exceeded")
            } else {
                self.raw() + other.raw()
            };
        Self::new(add_raw, Self::FRACTIONAL_BITS)
    }

    fn sub(&self, other: &Self) -> Self {
        let sub_raw =
            if DO_CHECKS {
                self.raw().checked_sub(other.raw())
                    .expect("numeric range exceeded")
            } else {
                self.raw() - other.raw()
            };
        Self::new(sub_raw, Self::FRACTIONAL_BITS)
    }

    fn log2(&self, lut: &Log2Lut) -> Log2D {
        if DO_CHECKS { assert_ne!(self.raw(), 0); }
        let leading_zeros = self.raw().leading_zeros() as u8;
        let raw_shifted = (self.raw() << leading_zeros)
            >> Self::TOTAL_BITS - Log2Lut::INDEX_BITS - 1;
        let result_fract = lut.log2_restricted(raw_shifted) as i32;
        let result_trunc =
            Self::INTEGRAL_BITS as i32 - leading_zeros as i32 - 1;
        let result = result_fract + (result_trunc << Log2D::FRACTIONAL_BITS);
        Log2D::new(result, Log2D::FRACTIONAL_BITS)
    }
}

pub mod fix_u32 {
    use DO_CHECKS;
    use super::{FixU32, FixU64, fix_u64};

    pub fn scaled_down(raw: u32, shift: u8) -> u32 {
        if DO_CHECKS { assert_ne!(shift, 0); }
        raw.saturating_add(1 << (shift - 1)) >> shift
    }

    pub fn mul<A: FixU32, B: FixU32, R: FixU32>(a: &A, b: &B) -> R {
        let mul_raw = a.raw() as u64 * b.raw() as u64;
        let total_fract_bits = A::FRACTIONAL_BITS + B::FRACTIONAL_BITS;
        if total_fract_bits > R::FRACTIONAL_BITS {
            let bits_diff = total_fract_bits - R::FRACTIONAL_BITS;
            let result_wide = fix_u64::scaled_down(mul_raw, bits_diff);
            if DO_CHECKS {
                assert_eq!((result_wide as u32) as u64, result_wide);
            }
            R::new(result_wide as u32, R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - total_fract_bits;
            let result_wide = mul_raw << bits_diff;
            if DO_CHECKS {
                assert_eq!((result_wide as u32) as u64, result_wide);
                assert_eq!(result_wide >> bits_diff, mul_raw);
            }
            R::new(result_wide as u32, R::FRACTIONAL_BITS)
        }
    }

    pub fn mul_wide<A: FixU32, B: FixU32, R: FixU64>(a: &A, b: &B) -> R {
        let mul_raw = a.raw() as u64 * b.raw() as u64;
        let total_fract_bits = A::FRACTIONAL_BITS + B::FRACTIONAL_BITS;
        if total_fract_bits > R::FRACTIONAL_BITS {
            let bits_diff = total_fract_bits - R::FRACTIONAL_BITS;
            R::new(fix_u64::scaled_down(mul_raw, bits_diff), R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - total_fract_bits;
            if DO_CHECKS {
                assert_eq!(mul_raw << bits_diff >> bits_diff, mul_raw);
            }
            R::new(mul_raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }
}

pub trait FixI64: FixedPoint<Raw=i64> {
    const TOTAL_BITS: u8 = 64;
    const INTEGRAL_BITS: u8 = Self::TOTAL_BITS - Self::FRACTIONAL_BITS;

    fn ulp() -> Self {
        Self::new_unchecked(1)
    }

    fn trunc(&self) -> Self::Raw {
        self.raw() >> Self::FRACTIONAL_BITS
    }

    fn to_fix_i64<R: FixI64>(&self) -> R {
        let raw = self.raw();
        if Self::FRACTIONAL_BITS > R::FRACTIONAL_BITS {
            let bits_diff = Self::FRACTIONAL_BITS - R::FRACTIONAL_BITS;
            R::new(fix_i64::scaled_down(raw, bits_diff), R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - Self::FRACTIONAL_BITS;
            if DO_CHECKS { assert_eq!(raw << bits_diff >> bits_diff, raw); }
            R::new(raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    fn add(&self, other: &Self) -> Self {
        let add_raw =
            if DO_CHECKS {
                self.raw().checked_add(other.raw())
                    .expect("numeric range exceeded")
            } else {
                self.raw() + other.raw()
            };
        Self::new(add_raw, Self::FRACTIONAL_BITS)
    }

    fn sub(&self, other: &Self) -> Self {
        let sub_raw =
            if DO_CHECKS {
                self.raw().checked_sub(other.raw())
                    .expect("numeric range exceeded")
            } else {
                self.raw() - other.raw()
            };
        Self::new(sub_raw, Self::FRACTIONAL_BITS)
    }

    fn neg(&self) -> Self {
        Self::new(-self.raw(), Self::FRACTIONAL_BITS)
    }
}

pub mod fix_i64 {
    use DO_CHECKS;

    pub fn scaled_down(raw: i64, shift: u8) -> i64 {
        if DO_CHECKS { assert_ne!(shift, 0); }
        raw.saturating_add(raw.signum() << (shift - 1))
            .max(i64::min_value() + 1) / (1 << shift)
    }
}

pub trait FixU64: FixedPoint<Raw=u64> {
    const TOTAL_BITS: u8 = 64;
    const INTEGRAL_BITS: u8 = Self::TOTAL_BITS - Self::FRACTIONAL_BITS;

    fn ulp() -> Self {
        Self::new_unchecked(1)
    }

    fn trunc(&self) -> Self::Raw {
        self.raw() >> Self::FRACTIONAL_BITS
    }

    fn to_fix_u64<R: FixU64>(&self) -> R {
        let raw = self.raw();
        if Self::FRACTIONAL_BITS > R::FRACTIONAL_BITS {
            let bits_diff = Self::FRACTIONAL_BITS - R::FRACTIONAL_BITS;
            R::new(fix_u64::scaled_down(raw, bits_diff), R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - Self::FRACTIONAL_BITS;
            if DO_CHECKS { assert_eq!(raw << bits_diff >> bits_diff, raw); }
            R::new(raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    fn add(&self, other: &Self) -> Self {
        let add_raw =
            if DO_CHECKS {
                self.raw().checked_add(other.raw())
                    .expect("numeric range exceeded")
            } else {
                self.raw() + other.raw()
            };
        Self::new(add_raw, Self::FRACTIONAL_BITS)
    }

    fn sub(&self, other: &Self) -> Self {
        let sub_raw =
            if DO_CHECKS {
                self.raw().checked_sub(other.raw())
                    .expect("numeric range exceeded")
            } else {
                self.raw() - other.raw()
            };
        Self::new(sub_raw, Self::FRACTIONAL_BITS)
    }

    fn log2(&self, lut: &Log2Lut) -> Log2D {
        if DO_CHECKS { assert_ne!(self.raw(), 0); }
        let leading_zeros = self.raw().leading_zeros() as u8;
        let raw_shifted = (self.raw() << leading_zeros)
            >> Self::TOTAL_BITS - Log2Lut::INDEX_BITS - 1;
        let result_fract = lut.log2_restricted(raw_shifted as u32) as i32;
        let result_trunc =
            Self::INTEGRAL_BITS as i32 - leading_zeros as i32 - 1;
        let result = result_fract + (result_trunc << Log2D::FRACTIONAL_BITS);
        Log2D::new(result, Log2D::FRACTIONAL_BITS)
    }
}

pub mod fix_u64 {
    use DO_CHECKS;

    pub fn scaled_down(raw: u64, shift: u8) -> u64 {
        if DO_CHECKS { assert_ne!(shift, 0); }
        raw.saturating_add(1 << (shift - 1)) >> shift
    }
}

impl<T: FixedPoint<Raw=i32>> FixI32 for T {}

impl<T: FixedPoint<Raw=u32>> FixU32 for T {}

impl<T: FixedPoint<Raw=i64>> FixI64 for T {}

impl<T: FixedPoint<Raw=u64>> FixU64 for T {}

pub trait FloatConversions {
    fn from_f32(input: f32) -> Self;
    fn from_f64(input: f64) -> Self;
    fn into_f32(self) -> f32;
    fn into_f64(self) -> f64;
}

impl FloatConversions for i32 {
    fn from_f32(input: f32) -> Self { input as Self }
    fn from_f64(input: f64) -> Self { input as Self }
    fn into_f32(self) -> f32 { self as f32 }
    fn into_f64(self) -> f64 { self as f64 }
}

impl FloatConversions for u32 {
    fn from_f32(input: f32) -> Self { input as Self }
    fn from_f64(input: f64) -> Self { input as Self }
    fn into_f32(self) -> f32 { self as f32 }
    fn into_f64(self) -> f64 { self as f64 }
}

impl FloatConversions for i64 {
    fn from_f32(input: f32) -> Self { input as Self }
    fn from_f64(input: f64) -> Self { input as Self }
    fn into_f32(self) -> f32 { self as f32 }
    fn into_f64(self) -> f64 { self as f64 }
}

impl FloatConversions for u64 {
    fn from_f32(input: f32) -> Self { input as Self }
    fn from_f64(input: f64) -> Self { input as Self }
    fn into_f32(self) -> f32 { self as f32 }
    fn into_f64(self) -> f64 { self as f64 }
}
