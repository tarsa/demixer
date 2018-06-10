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
use bit::Bit;
use fixed_point::FixedPoint;
use fixed_point::types::FractOnlyU32;

pub struct FixedSpeedEstimator(u16);

impl FixedSpeedEstimator {
    const BITS: u8 = 16;
    const LOG_RATE: u8 = 7;

    pub fn new(fract: u16) -> FixedSpeedEstimator {
        assert_ne!(fract, 0);
        FixedSpeedEstimator(fract)
    }

    /** Probability of bit 0 */
    pub fn prediction(&self) -> FractOnlyU32 {
        FractOnlyU32::new_unchecked(
            (self.0 as u32) << (FractOnlyU32::FRACTIONAL_BITS - Self::BITS))
    }

    pub fn update(&mut self, value: Bit) {
        let prediction = match value {
            Bit::Zero => {
                let error = (1u32 << Self::BITS) - self.0 as u32;
                let correction = error >> Self::LOG_RATE;
                self.0 + correction as u16
            }
            Bit::One => {
                self.0 - (self.0 >> Self::LOG_RATE)
            }
        };
        self.0 = prediction;
    }
}

impl Default for FixedSpeedEstimator {
    fn default() -> FixedSpeedEstimator {
        FixedSpeedEstimator(1u16 << (Self::BITS - 1))
    }
}
