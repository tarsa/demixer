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
use fixed_point::{FixedPoint, FixU32, fix_u32};
use fixed_point::types::FractOnlyU32;
use lut::estimator::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeceleratingEstimator(u32);

impl DeceleratingEstimator {
    pub const COUNT_BITS: u8 = 10;
    const PREDICTION_BITS: u8 = 32 - Self::COUNT_BITS;

    pub const MAX_COUNT: u16 = 1000;

    pub const INVALID: DeceleratingEstimator =
        DeceleratingEstimator(0);

    pub fn new() -> DeceleratingEstimator {
        let prediction = FractOnlyU32::new(1 << 30, 31);
        let count = 0;
        Self::make(prediction, count)
    }

    pub fn make(prediction: FractOnlyU32, count: u16)
                -> DeceleratingEstimator {
        assert!(FractOnlyU32::FRACTIONAL_BITS > Self::PREDICTION_BITS);
        let diff_bits = FractOnlyU32::FRACTIONAL_BITS - Self::PREDICTION_BITS;
        let prediction = (prediction.raw() + (1 << (diff_bits - 1)))
            >> diff_bits;
        let prediction =
            if prediction == 0 {
                1
            } else if prediction == 1 << (Self::PREDICTION_BITS) {
                (1 << (Self::PREDICTION_BITS)) - 1
            } else {
                prediction
            };
        assert!(prediction > 0 && prediction < (1 << Self::PREDICTION_BITS));
        assert!(count <= Self::MAX_COUNT);
        let raw = (prediction << Self::COUNT_BITS) | (count as u32);
        DeceleratingEstimator(raw)
    }

    /** Probability of bit 0 */
    pub fn prediction(&self) -> FractOnlyU32 {
        let raw = self.0 >> Self::COUNT_BITS;
        assert!(FractOnlyU32::FRACTIONAL_BITS > Self::PREDICTION_BITS);
        let shift = FractOnlyU32::FRACTIONAL_BITS - Self::PREDICTION_BITS;
        FractOnlyU32::new_unchecked(raw << shift)
    }

    pub fn usage_count(&self) -> u16 {
        (self.0 as u16) & ((1 << Self::COUNT_BITS) - 1)
    }

    pub fn update(&mut self, value: Bit, lut: &DeceleratingEstimatorLut) {
        let prediction = self.prediction();
        let count = self.usage_count();
        let factor = lut[count];
        let prediction = match value {
            Bit::Zero => {
                let error = FractOnlyU32::ONE_UNSAFE.sub(&prediction);
                let correction: FractOnlyU32 = fix_u32::mul(&error, &factor);
                prediction.add(&correction)
            }
            Bit::One => {
                let error = prediction;
                let correction: FractOnlyU32 = fix_u32::mul(&error, &factor);
                prediction.sub(&correction)
            }
        };
        let count = (count + 1).min(Self::MAX_COUNT);
        *self = Self::make(prediction, count);
    }
}
