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
use super::*;

use DO_CHECKS;
use lut::log2::Log2Lut;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NoFractI32(i32);

impl FixedPoint for NoFractI32 {
    type Raw = i32;
    fn raw(&self) -> Self::Raw { self.0 }
    fn new_unchecked(raw: Self::Raw) -> Self { NoFractI32(raw) }

    const FRACTIONAL_BITS: u8 = 0;
}

impl NoFractI32 {
    pub const ZERO: Self = NoFractI32(0);
    pub const ONE: Self = NoFractI32(1i32 << Self::FRACTIONAL_BITS);

    pub fn to_unsigned(&self) -> NoFractU32 {
        if DO_CHECKS { assert!(self.raw() >= 0); }
        NoFractU32::new(self.raw() as u32, Self::FRACTIONAL_BITS)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NoFractU32(u32);

impl FixedPoint for NoFractU32 {
    type Raw = u32;
    fn raw(&self) -> Self::Raw { self.0 }
    fn new_unchecked(raw: Self::Raw) -> Self { NoFractU32(raw) }

    const FRACTIONAL_BITS: u8 = 0;
}

impl NoFractU32 {
    pub const ZERO: Self = NoFractU32(0);
    pub const ONE: Self = NoFractU32(1u32 << Self::FRACTIONAL_BITS);

    pub fn to_signed(&self) -> NoFractI32 {
        if DO_CHECKS { assert!(self.raw() <= <i32>::max_value() as u32); }
        NoFractI32::new(self.raw() as i32, Self::FRACTIONAL_BITS)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NoFractI64(i64);

impl FixedPoint for NoFractI64 {
    type Raw = i64;
    fn raw(&self) -> Self::Raw { self.0 }
    fn new_unchecked(raw: Self::Raw) -> Self { NoFractI64(raw) }

    const FRACTIONAL_BITS: u8 = 0;
}

impl NoFractI64 {
    pub const ZERO: Self = NoFractI64(0);
    pub const ONE: Self = NoFractI64(1i64 << Self::FRACTIONAL_BITS);

    pub fn to_unsigned(&self) -> NoFractU64 {
        if DO_CHECKS { assert!(self.raw() >= 0); }
        NoFractU64::new(self.raw() as u64, Self::FRACTIONAL_BITS)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NoFractU64(u64);

impl FixedPoint for NoFractU64 {
    type Raw = u64;
    fn raw(&self) -> Self::Raw { self.0 }
    fn new_unchecked(raw: Self::Raw) -> Self { NoFractU64(raw) }

    const FRACTIONAL_BITS: u8 = 0;
}

impl NoFractU64 {
    pub const ZERO: Self = NoFractU64(0);
    pub const ONE: Self = NoFractU64(1u64 << Self::FRACTIONAL_BITS);

    pub fn to_signed(&self) -> NoFractI64 {
        if DO_CHECKS { assert!(self.raw() <= <i64>::max_value() as u64); }
        NoFractI64::new(self.raw() as i64, Self::FRACTIONAL_BITS)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FractOnlyI32(i32);

impl FixedPoint for FractOnlyI32 {
    type Raw = i32;
    fn raw(&self) -> Self::Raw { self.0 }
    fn new_unchecked(raw: Self::Raw) -> Self { FractOnlyI32(raw) }
    fn within_bounds(&self) -> bool { self.raw() >= 0 }

    const FRACTIONAL_BITS: u8 = 31;
}

impl FractOnlyI32 {
    pub const ZERO: Self = FractOnlyI32(0);
    pub const HALF: Self = FractOnlyI32(1i32 << (Self::FRACTIONAL_BITS - 1));
    pub const ONE_UNSAFE: Self = FractOnlyI32(1i32 << Self::FRACTIONAL_BITS);

    /// Returns: (1 - self)
    pub fn flip(&self) -> Self {
        Self::ONE_UNSAFE.sub(self)
    }

    pub fn to_unsigned(&self) -> FractOnlyU32 {
        FractOnlyU32::new(self.raw() as u32, Self::FRACTIONAL_BITS)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FractOnlyU32(u32);

impl FixedPoint for FractOnlyU32 {
    type Raw = u32;
    fn raw(&self) -> Self::Raw { self.0 }
    fn new_unchecked(raw: Self::Raw) -> Self { FractOnlyU32(raw) }
    fn within_bounds(&self) -> bool { (self.raw() >> 31) == 0 }

    const FRACTIONAL_BITS: u8 = 31;
}

impl FractOnlyU32 {
    pub const ZERO: Self = FractOnlyU32(0);
    pub const HALF: Self = FractOnlyU32(1u32 << (Self::FRACTIONAL_BITS - 1));
    pub const ONE_UNSAFE: Self = FractOnlyU32(1u32 << Self::FRACTIONAL_BITS);

    /// Returns: (1 - self)
    pub fn flip(&self) -> Self {
        Self::ONE_UNSAFE.sub(self)
    }

    pub fn to_signed(&self) -> FractOnlyI32 {
        FractOnlyI32::new(self.raw() as i32, Self::FRACTIONAL_BITS)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FractOnlyI64(i64);

impl FixedPoint for FractOnlyI64 {
    type Raw = i64;
    fn raw(&self) -> Self::Raw { self.0 }
    fn new_unchecked(raw: Self::Raw) -> Self { FractOnlyI64(raw) }
    fn within_bounds(&self) -> bool { self.raw() >= 0 }

    const FRACTIONAL_BITS: u8 = 63;
}

impl FractOnlyI64 {
    pub const ZERO: Self = FractOnlyI64(0);
    pub const HALF: Self = FractOnlyI64(1i64 << (Self::FRACTIONAL_BITS - 1));
    pub const ONE_UNSAFE: Self = FractOnlyI64(1i64 << Self::FRACTIONAL_BITS);

    /// Returns: (1 - self)
    pub fn flip(&self) -> Self {
        Self::ONE_UNSAFE.sub(self)
    }

    pub fn to_unsigned(&self) -> FractOnlyU64 {
        FractOnlyU64::new(self.raw() as u64, Self::FRACTIONAL_BITS)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FractOnlyU64(u64);

impl FixedPoint for FractOnlyU64 {
    type Raw = u64;
    fn raw(&self) -> Self::Raw { self.0 }
    fn new_unchecked(raw: Self::Raw) -> Self { FractOnlyU64(raw) }
    fn within_bounds(&self) -> bool { (self.raw() >> 63) == 0 }

    const FRACTIONAL_BITS: u8 = 63;
}

impl FractOnlyU64 {
    pub const ZERO: Self = FractOnlyU64(0);
    pub const HALF: Self = FractOnlyU64(1u64 << (Self::FRACTIONAL_BITS - 1));
    pub const ONE_UNSAFE: Self = FractOnlyU64(1u64 << Self::FRACTIONAL_BITS);

    /// Returns: (1 - self)
    pub fn flip(&self) -> Self {
        Self::ONE_UNSAFE.sub(self)
    }

    pub fn to_signed(&self) -> FractOnlyI64 {
        FractOnlyI64::new(self.raw() as i64, Self::FRACTIONAL_BITS)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Log2D(i32);

impl FixedPoint for Log2D {
    type Raw = i32;
    fn raw(&self) -> i32 { self.0 }
    fn new_unchecked(raw: i32) -> Self { Log2D(raw) }

    const FRACTIONAL_BITS: u8 = Log2Lut::INDEX_BITS;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Log2Q(i64);

impl FixedPoint for Log2Q {
    type Raw = i64;
    fn raw(&self) -> i64 { self.0 }
    fn new_unchecked(raw: i64) -> Self { Log2Q(raw) }

    const FRACTIONAL_BITS: u8 = Log2Lut::INDEX_BITS;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StretchedProbD(i32);

impl FixedPoint for StretchedProbD {
    type Raw = i32;
    fn raw(&self) -> i32 { self.0 }
    fn new_unchecked(raw: i32) -> Self { StretchedProbD(raw) }
    fn within_bounds(&self) -> bool {
        let raw = self.raw();
        let limit = Self::ABSOLUTE_LIMIT;
        let scale = Self::FRACTIONAL_BITS;
        raw >= (-limit << scale) && raw <= (limit << scale)
    }

    const FRACTIONAL_BITS: u8 = 21;
}

impl StretchedProbD {
    pub const ZERO: Self = StretchedProbD(0);

    pub const ABSOLUTE_LIMIT: i32 = 12;

    pub const MAX: Self =
        StretchedProbD(Self::ABSOLUTE_LIMIT << Self::FRACTIONAL_BITS);
    pub const MIN: Self =
        StretchedProbD(-(Self::ABSOLUTE_LIMIT << Self::FRACTIONAL_BITS));

    pub fn intervals_count(stretched_fract_index_bits: u8) -> i32 {
        2 * Self::ABSOLUTE_LIMIT << stretched_fract_index_bits
    }

    pub fn interval_stops_count(stretched_fract_index_bits: u8) -> i32 {
        1 + Self::intervals_count(stretched_fract_index_bits)
    }

    pub fn to_interval_index(&self, fract_index_bits: u8) -> i32 {
        let offset = Self::ABSOLUTE_LIMIT << fract_index_bits;
        offset + (self.raw() >> Self::FRACTIONAL_BITS - fract_index_bits)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StretchedProbQ(i64);

impl FixedPoint for StretchedProbQ {
    type Raw = i64;
    fn raw(&self) -> i64 { self.0 }
    fn new_unchecked(raw: i64) -> Self { StretchedProbQ(raw) }

    const FRACTIONAL_BITS: u8 = 40;
}

impl StretchedProbQ {
    pub const ZERO: Self = StretchedProbQ(0);

    pub const ABSOLUTE_LIMIT: i64 = StretchedProbD::ABSOLUTE_LIMIT as i64;

    pub const MAX: Self =
        StretchedProbQ(Self::ABSOLUTE_LIMIT << Self::FRACTIONAL_BITS);
    pub const MIN: Self =
        StretchedProbQ(-(Self::ABSOLUTE_LIMIT << Self::FRACTIONAL_BITS));

    pub fn clamped(self) -> Self {
        self.max(Self::MIN).min(Self::MAX)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MixerWeight(i32);

impl FixedPoint for MixerWeight {
    type Raw = i32;
    fn raw(&self) -> i32 { self.0 }
    fn new_unchecked(raw: i32) -> Self { MixerWeight(raw) }

    const FRACTIONAL_BITS: u8 = 21;
}

impl MixerWeight {
    pub const ZERO: Self = MixerWeight(0);
    pub const ONE: Self = MixerWeight(1i32 << Self::FRACTIONAL_BITS);

    pub const ABSOLUTE_LIMIT: i32 = 8;

    pub const MAX: Self =
        MixerWeight(Self::ABSOLUTE_LIMIT << Self::FRACTIONAL_BITS);
    pub const MIN: Self =
        MixerWeight(-(Self::ABSOLUTE_LIMIT << Self::FRACTIONAL_BITS));

    pub fn clamped(self) -> Self {
        self.max(Self::MIN).min(Self::MAX)
    }
}
