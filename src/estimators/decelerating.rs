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
    pub const LENGTH_BITS: u8 = 10;
    const PREDICTION_BITS: u8 = 32 - Self::LENGTH_BITS;

    pub const MAX_LENGTH: u16 = 1000;

    pub const INVALID: DeceleratingEstimator =
        DeceleratingEstimator(0);

    pub fn new() -> DeceleratingEstimator {
        let prediction = FractOnlyU32::new(1 << 30, 31);
        let length = 0;
        Self::make(prediction, length)
    }

    pub fn make(prediction: FractOnlyU32, length: u16)
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
        assert!(length <= Self::MAX_LENGTH);
        let raw = (prediction << Self::LENGTH_BITS) | (length as u32);
        DeceleratingEstimator(raw)
    }

    /** Probability of bit 0 */
    pub fn prediction(&self) -> FractOnlyU32 {
        let raw = self.0 >> Self::LENGTH_BITS;
        assert!(FractOnlyU32::FRACTIONAL_BITS > Self::PREDICTION_BITS);
        let shift = FractOnlyU32::FRACTIONAL_BITS - Self::PREDICTION_BITS;
        FractOnlyU32::new_unchecked(raw << shift)
    }

    pub fn length(&self) -> u16 {
        (self.0 as u16) & ((1 << Self::LENGTH_BITS) - 1)
    }

    pub fn update(&mut self, value: Bit, lut: &DeceleratingEstimatorLut) {
        let prediction = self.prediction();
        let length = self.length();
        let factor = lut[length];
        let prediction = match value {
            Bit::Zero => {
                let error = FractOnlyU32::new_unchecked(
                    1u32 << FractOnlyU32::FRACTIONAL_BITS).sub(&prediction);
                let correction: FractOnlyU32 = fix_u32::mul(&error, &factor);
                prediction.add(&correction)
            }
            Bit::One => {
                let error = prediction;
                let correction: FractOnlyU32 = fix_u32::mul(&error, &factor);
                prediction.sub(&correction)
            }
        };
        let length = (length + 1).min(Self::MAX_LENGTH);
        *self = Self::make(prediction, length);
    }
}
