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
use std::ops::Index;

use bit::Bit;
use estimators::decelerating::DeceleratingEstimator;
use fixed_point::FixedPoint;
use fixed_point::types::FractOnlyU32;

pub struct DeceleratingEstimatorRates(
    [FractOnlyU32; 1usize << DeceleratingEstimator::COUNT_BITS]);

impl DeceleratingEstimatorRates {
    pub fn make_default() -> DeceleratingEstimatorRates {
        Self::make(1, 2)
    }

    pub fn make(factor: u32, addend: u32) -> DeceleratingEstimatorRates {
        let mut array = [FractOnlyU32::new_unchecked(0);
            1 << DeceleratingEstimator::COUNT_BITS];
        for index in 0..array.len() {
            let denominator = (index as u32) * factor + addend;
            array[index] = FractOnlyU32::new_unchecked(
                (1u32 << FractOnlyU32::FRACTIONAL_BITS) / denominator);
        }
        DeceleratingEstimatorRates(array)
    }
}

impl Index<u16> for DeceleratingEstimatorRates {
    type Output = FractOnlyU32;

    fn index(&self, index: u16) -> &FractOnlyU32 {
        &self.0[index as usize]
    }
}

pub struct DeceleratingEstimatorCache(
    [DeceleratingEstimator; 1usize << DeceleratingEstimator::COUNT_BITS]);

impl DeceleratingEstimatorCache {
    pub fn new(lut: &DeceleratingEstimatorRates) -> DeceleratingEstimatorCache {
        let mut estimator = DeceleratingEstimator::new();
        let mut array = [DeceleratingEstimator::INVALID;
            1 << DeceleratingEstimator::COUNT_BITS];
        for index in 0..array.len() {
            let mut current = estimator;
            current.update(Bit::One, lut);
            array[index] = current;
            estimator.update(Bit::Zero, lut);
        }
        DeceleratingEstimatorCache(array)
    }

    pub fn for_new_node(&self, last_bit: Bit, opposite_bit_run_length: u16)
                        -> DeceleratingEstimator {
        assert!(opposite_bit_run_length <= DeceleratingEstimator::MAX_COUNT);
        if last_bit.is_1() {
            self.0[opposite_bit_run_length as usize]
        } else {
            let flipped = self.0[opposite_bit_run_length as usize];
            let prediction = flipped.prediction().flip();
            DeceleratingEstimator::make(prediction, flipped.usage_count())
        }
    }
}
