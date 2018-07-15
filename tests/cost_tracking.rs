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
extern crate demixer;

use demixer::estimators::cost::CostTracker;
use demixer::fixed_point::{FixedPoint, FixI32, FixU32};
use demixer::fixed_point::types::{NoFractI32, Log2D};
use demixer::lut::cost::CostTrackersLut;
use demixer::lut::log2::Log2Lut;
use demixer::lut::estimator::DeceleratingEstimatorRates;
use demixer::bit::Bit;
use demixer::estimators::decelerating::DeceleratingEstimator;
use demixer::coding::FinalProbability;

#[test]
fn initial_cost_corresponds_to_one_bit_costs_series() {
    let old_tracker = CostTracker::INITIAL;
    let new_tracker = old_tracker.updated(NoFractI32::ONE.to_fix_i32());
    assert_eq!(old_tracker.raw(), new_tracker.raw());
}

#[test]
fn cost_decays_by_decay_rate() {
    let old_tracker = CostTracker::INITIAL;
    let new_tracker = old_tracker.updated(minimum_cost());
    assert_eq!(old_tracker.raw() -
                   (old_tracker.raw() >> CostTracker::DECAY_SCALE),
               new_tracker.raw() - 1);
}

#[test]
fn older_cost_has_less_weight_than_newer() {
    let tracker01 = CostTracker::INITIAL
        .updated(minimum_cost())
        .updated(NoFractI32::ONE.to_fix_i32());
    let tracker10 = CostTracker::INITIAL
        .updated(NoFractI32::ONE.to_fix_i32())
        .updated(minimum_cost());
    assert!(tracker01.raw() > tracker10.raw());

    let tracker12 = CostTracker::INITIAL
        .updated(NoFractI32::ONE.to_fix_i32())
        .updated(NoFractI32::new(2, 0).to_fix_i32());
    let tracker21 = CostTracker::INITIAL
        .updated(NoFractI32::new(2, 0).to_fix_i32())
        .updated(NoFractI32::ONE.to_fix_i32());
    assert!(tracker12.raw() > tracker21.raw());
}

#[test]
fn cost_trackers_lut_is_good() {
    let log2_lut = Log2Lut::new();
    let rates_lut = DeceleratingEstimatorRates::make_default();
    let cost_tracker_lut = CostTrackersLut::new(&log2_lut, &rates_lut);
    let mut cost_tracker = CostTracker::INITIAL;
    let mut estimator = DeceleratingEstimator::new();
    for run_length in 0..=DeceleratingEstimator::MAX_COUNT {
        let prediction = estimator.prediction();
        let final_probability: FinalProbability = prediction.to_fix_u32();
        assert_eq!(
            cost_tracker.updated(
                final_probability.estimate_cost(Bit::One, &log2_lut)),
            cost_tracker_lut.for_new_node(run_length));
        cost_tracker = cost_tracker.updated(
            final_probability.estimate_cost(Bit::Zero, &log2_lut));
        estimator.update(Bit::Zero, &rates_lut);
    }
}

fn minimum_cost() -> Log2D {
    Log2D::new(1, Log2D::FRACTIONAL_BITS)
}
