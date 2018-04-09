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

use lut::log2::LOG2_ACCURATE_BITS;

#[derive(Debug, PartialEq, Eq)]
pub struct Log2D(i32);

impl FixedPoint for Log2D {
    type Raw = i32;
    fn raw(&self) -> i32 { self.0 }
    fn new_raw(raw: i32) -> Self { Log2D(raw) }

    const FRACTIONAL_BITS: u8 = LOG2_ACCURATE_BITS;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Log2Q(i64);

impl FixedPoint for Log2Q {
    type Raw = i64;
    fn raw(&self) -> i64 { self.0 }
    fn new_raw(raw: i64) -> Self { Log2Q(raw) }

    const FRACTIONAL_BITS: u8 = LOG2_ACCURATE_BITS;
}
