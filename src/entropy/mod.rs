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
use fixed_point::{FixedPoint, FixI32, FixU32};
use fixed_point::types::Log2D;
use lut::log2::Log2Lut;

pub mod decoder;
pub mod encoder;

/** Probability of bit 0 */
#[derive(Clone)]
pub struct FinalProbability(u32);

impl FinalProbability {
    pub fn estimate_cost(&self, is_1: bool, lut: &Log2Lut) -> Log2D {
        let probability =
            if is_1 {
                Self::new((1 << 12) - self.raw(), 12)
            } else {
                self.clone()
            };
        probability.log2(lut).neg()
    }
}

impl FixedPoint for FinalProbability {
    type Raw = u32;
    fn raw(&self) -> u32 { self.0 }
    fn new_raw(raw: u32) -> Self { FinalProbability(raw) }
    fn within_bounds(&self) -> bool {
        let raw = self.0;
        (raw > 0) && (raw < 1 << Self::FRACTIONAL_BITS)
    }

    const FRACTIONAL_BITS: u8 = 12;
}
