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
use coding::FinalProbability;
use estimators::cost::CostTracker;
use estimators::decelerating::DeceleratingEstimator;
use fixed_point::FixU32;
use lut::estimator::DeceleratingEstimatorRates;
use lut::log2::Log2Lut;

pub struct CostTrackersLut(
    [CostTracker; 1usize << DeceleratingEstimator::COUNT_BITS]);

impl CostTrackersLut {
    const COUNT: usize = 1usize << DeceleratingEstimator::COUNT_BITS;

    pub fn new(log2_lut: &Log2Lut,
               rates_lut: &DeceleratingEstimatorRates) -> Self {
        let mut cost_tracker = CostTracker::INITIAL;
        let mut estimator = DeceleratingEstimator::new();
        let mut result = [cost_tracker; Self::COUNT];
        for index in 0..Self::COUNT {
            let prediction = estimator.prediction();
            let final_probability: FinalProbability = prediction.to_fix_u32();
            let opposite_cost =
                final_probability.estimate_cost(Bit::One, log2_lut);
            result[index] = cost_tracker.updated(opposite_cost);
            let cost = final_probability.estimate_cost(Bit::Zero, log2_lut);
            cost_tracker = cost_tracker.updated(cost);
            estimator.update(Bit::Zero, rates_lut);
        }
        CostTrackersLut(result)
    }

    pub fn for_new_node(&self, opposite_bit_run_length: u16) -> CostTracker {
        let index = (opposite_bit_run_length as usize).min(Self::COUNT - 1);
        self.0[index]
    }
}
