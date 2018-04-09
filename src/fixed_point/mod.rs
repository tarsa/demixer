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

use fixed_point::types::Log2D;
use lut::log2::Log2Lut;

pub trait FixedPoint where Self: Sized {
    type Raw: AsFloat;

    fn raw(&self) -> Self::Raw;
    fn new_raw(raw: Self::Raw) -> Self;
    fn new(raw: Self::Raw, fractional_bits: u8) -> Self {
        assert_eq!(fractional_bits, Self::FRACTIONAL_BITS);
        let result = Self::new_raw(raw);
        assert!(result.within_bounds());
        result
    }
    fn within_bounds(&self) -> bool { true }

    const FRACTIONAL_BITS: u8;

    fn as_f32(&self) -> f32 {
        self.raw().into_f32() / 2f32.powi(Self::FRACTIONAL_BITS as i32)
    }

    fn as_f64(&self) -> f64 {
        self.raw().into_f64() / 2f64.powi(Self::FRACTIONAL_BITS as i32)
    }
}

pub trait FixI32: FixedPoint<Raw=i32> {
    const TOTAL_BITS: u8 = 32;
    const INTEGRAL_BITS: u8 = Self::TOTAL_BITS - Self::FRACTIONAL_BITS;

    fn trunc(&self) -> Self::Raw {
        self.raw() >> Self::FRACTIONAL_BITS
    }

    fn to_fix_i32<R: FixI32>(&self) -> R {
        let raw = self.raw();
        if Self::FRACTIONAL_BITS >= R::FRACTIONAL_BITS {
            let bits_diff = Self::FRACTIONAL_BITS - R::FRACTIONAL_BITS;
            R::new(raw >> bits_diff, R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - Self::FRACTIONAL_BITS;
            assert_eq!(raw << bits_diff >> bits_diff, raw);
            R::new(raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    fn to_fix_i64<R: FixI64>(&self) -> R {
        let raw = self.raw() as i64;
        if Self::FRACTIONAL_BITS >= R::FRACTIONAL_BITS {
            let bits_diff = Self::FRACTIONAL_BITS - R::FRACTIONAL_BITS;
            R::new(raw >> bits_diff, R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - Self::FRACTIONAL_BITS;
            assert_eq!(raw << bits_diff >> bits_diff, raw);
            R::new(raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    fn neg(&self) -> Self {
        Self::new(-self.raw(), Self::FRACTIONAL_BITS)
    }
}

pub mod fix_i32 {
    use super::{FixI32, FixI64};

    pub fn mul<A: FixI32, B: FixI32, R: FixI32>(a: &A, b: &B) -> R {
        let mul_raw = a.raw() as i64 * b.raw() as i64;
        let total_fract_bits = A::FRACTIONAL_BITS + B::FRACTIONAL_BITS;
        if total_fract_bits >= R::FRACTIONAL_BITS {
            let bits_diff = total_fract_bits - R::FRACTIONAL_BITS;
            let result_wide = mul_raw >> bits_diff;
            assert_eq!((result_wide as i32) as i64, result_wide);
            R::new(result_wide as i32, R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - total_fract_bits;
            let result_wide = mul_raw << bits_diff;
            assert_eq!((result_wide as i32) as i64, result_wide);
            assert_eq!(result_wide >> bits_diff, mul_raw);
            R::new(result_wide as i32, R::FRACTIONAL_BITS)
        }
    }

    pub fn mul_wide<A: FixI32, B: FixI32, R: FixI64>(a: &A, b: &B) -> R {
        let mul_raw = a.raw() as i64 * b.raw() as i64;
        let total_fract_bits = A::FRACTIONAL_BITS + B::FRACTIONAL_BITS;
        if total_fract_bits >= R::FRACTIONAL_BITS {
            let bits_diff = total_fract_bits - R::FRACTIONAL_BITS;
            R::new(mul_raw >> bits_diff, R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - total_fract_bits;
            assert_eq!(mul_raw << bits_diff >> bits_diff, mul_raw);
            R::new(mul_raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    pub fn add<T: FixI32>(a: &T, b: &T) -> T {
        let add_raw = a.raw().checked_add(b.raw()).expect("overflow");
        T::new(add_raw, T::FRACTIONAL_BITS)
    }
}

pub trait FixU32: FixedPoint<Raw=u32> {
    const TOTAL_BITS: u8 = 32;
    const INTEGRAL_BITS: u8 = Self::TOTAL_BITS - Self::FRACTIONAL_BITS;

    fn trunc(&self) -> Self::Raw {
        self.raw() >> Self::FRACTIONAL_BITS
    }

    fn to_fix_u32<R: FixU32>(&self) -> R {
        let raw = self.raw();
        if Self::FRACTIONAL_BITS >= R::FRACTIONAL_BITS {
            let bits_diff = Self::FRACTIONAL_BITS - R::FRACTIONAL_BITS;
            R::new(raw >> bits_diff, R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - Self::FRACTIONAL_BITS;
            assert_eq!(raw << bits_diff >> bits_diff, raw);
            R::new(raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    fn to_fix_u64<R: FixU64>(&self) -> R {
        let raw = self.raw() as u64;
        if Self::FRACTIONAL_BITS >= R::FRACTIONAL_BITS {
            let bits_diff = Self::FRACTIONAL_BITS - R::FRACTIONAL_BITS;
            R::new(raw >> bits_diff, R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - Self::FRACTIONAL_BITS;
            assert_eq!(raw << bits_diff >> bits_diff, raw);
            R::new(raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    fn log2(&self, lut: &Log2Lut) -> Log2D {
        assert_ne!(self.raw(), 0);
        let leading_zeros = self.raw().leading_zeros() as u8;
        let raw_shifted = (self.raw() << leading_zeros)
            >> Self::TOTAL_BITS - Log2D::FRACTIONAL_BITS - 1;
        let result_fract = lut.log2_restricted(raw_shifted) as i32;
        let result_trunc =
            Self::INTEGRAL_BITS as i32 - leading_zeros as i32 - 1;
        let result = result_fract + (result_trunc << Log2D::FRACTIONAL_BITS);
        Log2D::new(result, Log2D::FRACTIONAL_BITS)
    }
}

pub mod fix_u32 {
    use super::{FixU32, FixU64};

    pub fn mul<A: FixU32, B: FixU32, R: FixU32>(a: &A, b: &B) -> R {
        let mul_raw = a.raw() as u64 * b.raw() as u64;
        let total_fract_bits = A::FRACTIONAL_BITS + B::FRACTIONAL_BITS;
        if total_fract_bits >= R::FRACTIONAL_BITS {
            let bits_diff = total_fract_bits - R::FRACTIONAL_BITS;
            let result_wide = mul_raw >> bits_diff;
            assert_eq!((result_wide as u32) as u64, result_wide);
            R::new(result_wide as u32, R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - total_fract_bits;
            let result_wide = mul_raw << bits_diff;
            assert_eq!((result_wide as u32) as u64, result_wide);
            assert_eq!(result_wide >> bits_diff, mul_raw);
            R::new(result_wide as u32, R::FRACTIONAL_BITS)
        }
    }

    pub fn mul_wide<A: FixU32, B: FixU32, R: FixU64>(a: &A, b: &B) -> R {
        let mul_raw = a.raw() as u64 * b.raw() as u64;
        let total_fract_bits = A::FRACTIONAL_BITS + B::FRACTIONAL_BITS;
        if total_fract_bits >= R::FRACTIONAL_BITS {
            let bits_diff = total_fract_bits - R::FRACTIONAL_BITS;
            R::new(mul_raw >> bits_diff, R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - total_fract_bits;
            assert_eq!(mul_raw << bits_diff >> bits_diff, mul_raw);
            R::new(mul_raw << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    pub fn add<T: FixU32>(a: &T, b: &T) -> T {
        let add_raw = a.raw().checked_add(b.raw()).expect("overflow");
        T::new(add_raw, T::FRACTIONAL_BITS)
    }
}

pub trait FixI64: FixedPoint<Raw=i64> {
    const TOTAL_BITS: u8 = 64;
    const INTEGRAL_BITS: u8 = Self::TOTAL_BITS - Self::FRACTIONAL_BITS;

    fn trunc(&self) -> Self::Raw {
        self.raw() >> Self::FRACTIONAL_BITS
    }

    fn to_fix_i64<R: FixI64>(&self) -> R {
        if Self::FRACTIONAL_BITS >= R::FRACTIONAL_BITS {
            let bits_diff = Self::FRACTIONAL_BITS - R::FRACTIONAL_BITS;
            R::new(self.raw() >> bits_diff, R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - Self::FRACTIONAL_BITS;
            assert_eq!(self.raw() << bits_diff >> bits_diff, self.raw());
            R::new(self.raw() << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    fn neg(&self) -> Self {
        Self::new(-self.raw(), Self::FRACTIONAL_BITS)
    }
}

pub mod fix_i64 {
    use super::FixI64;

    pub fn add<T: FixI64>(a: &T, b: &T) -> T {
        let add_raw = a.raw().checked_add(b.raw()).expect("overflow");
        T::new(add_raw, T::FRACTIONAL_BITS)
    }
}

pub trait FixU64: FixedPoint<Raw=u64> {
    const TOTAL_BITS: u8 = 64;
    const INTEGRAL_BITS: u8 = Self::TOTAL_BITS - Self::FRACTIONAL_BITS;

    fn trunc(&self) -> Self::Raw {
        self.raw() >> Self::FRACTIONAL_BITS
    }

    fn to_fix_u64<R: FixU64>(&self) -> R {
        if Self::FRACTIONAL_BITS >= R::FRACTIONAL_BITS {
            let bits_diff = Self::FRACTIONAL_BITS - R::FRACTIONAL_BITS;
            R::new(self.raw() >> bits_diff, R::FRACTIONAL_BITS)
        } else {
            let bits_diff = R::FRACTIONAL_BITS - Self::FRACTIONAL_BITS;
            assert_eq!(self.raw() << bits_diff >> bits_diff, self.raw());
            R::new(self.raw() << bits_diff, R::FRACTIONAL_BITS)
        }
    }

    fn log2(&self, lut: &Log2Lut) -> Log2D {
        assert_ne!(self.raw(), 0);
        let leading_zeros = self.raw().leading_zeros() as u8;
        let raw_shifted = (self.raw() << leading_zeros)
            >> Self::TOTAL_BITS - Log2D::FRACTIONAL_BITS - 1;
        let result_fract = lut.log2_restricted(raw_shifted as u32) as i32;
        let result_trunc =
            Self::INTEGRAL_BITS as i32 - leading_zeros as i32 - 1;
        let result = result_fract + (result_trunc << Log2D::FRACTIONAL_BITS);
        Log2D::new(result, Log2D::FRACTIONAL_BITS)
    }
}

pub mod fix_u64 {
    use super::FixU64;

    pub fn add<T: FixU64>(a: &T, b: &T) -> T {
        let add_raw = a.raw().checked_add(b.raw()).expect("overflow");
        T::new(add_raw, T::FRACTIONAL_BITS)
    }
}

impl<T: FixedPoint<Raw=i32>> FixI32 for T {}

impl<T: FixedPoint<Raw=u32>> FixU32 for T {}

impl<T: FixedPoint<Raw=i64>> FixI64 for T {}

impl<T: FixedPoint<Raw=u64>> FixU64 for T {}

pub trait AsFloat {
    fn into_f32(self) -> f32;
    fn into_f64(self) -> f64;
}

impl AsFloat for i32 {
    fn into_f32(self) -> f32 { self as f32 }
    fn into_f64(self) -> f64 { self as f64 }
}

impl AsFloat for u32 {
    fn into_f32(self) -> f32 { self as f32 }
    fn into_f64(self) -> f64 { self as f64 }
}

impl AsFloat for i64 {
    fn into_f32(self) -> f32 { self as f32 }
    fn into_f64(self) -> f64 { self as f64 }
}

impl AsFloat for u64 {
    fn into_f32(self) -> f32 { self as f32 }
    fn into_f64(self) -> f64 { self as f64 }
}
