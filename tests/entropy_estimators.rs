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

use demixer::bit::Bit;
use demixer::entropy::FinalProbability;
use demixer::estimators::decelerating::*;
use demixer::estimators::fixed_speed::FixedSpeedEstimator;
use demixer::fixed_point::{FixedPoint, FixI32, FixU32, FixI64};
use demixer::fixed_point::types::Log2Q;
use demixer::lut::log2::{LOG2_ACCURATE_BITS, Log2Lut, make_log2_lut};

#[test]
fn decelerating_estimator_is_good() {
    let lut = DeceleratingEstimatorLut::make_default();
    let log_lut = make_log2_lut();
    for x in -16..16 + 1 {
        let max_overhead = (1 + (x as i32).abs()) as f64 / 100.0;
        let power = (-x as f64) / 2.0;
        let probability = 1f64 / (1f64 + power.exp());
        check_decelerating_estimator_single(probability, &lut, &log_lut,
                                            max_overhead);
    }
}

#[test]
fn fixed_speed_estimator_is_good() {
    let log_lut = make_log2_lut();
    for x in -14..14 + 1 {
        let max_overhead = (2 + (x as i32).abs()) as f64 / 100.0;
        let power = (-x as f64) / 2.0;
        let probability = 1f64 / (1f64 + power.exp());
        check_fixed_speed_estimator_single(probability, &log_lut, max_overhead);
    }
}

fn check_decelerating_estimator_single(probability: f64,
                                       lut: &DeceleratingEstimatorLut,
                                       log_lut: &Log2Lut,
                                       max_overhead: f64) {
    assert!(probability > 0.0 && probability < 1.0);
    let mut estimator = DeceleratingEstimator::new();
    let mut total_cost = Log2Q::new(0, LOG2_ACCURATE_BITS);
    let mut zeros = 0;
    let mut accumulator = probability;
    let warm_up = DeceleratingEstimator::MAX_LENGTH;
    let measured = warm_up * 10;
    let total = warm_up + measured;
    for iteration in 0..total {
        accumulator += probability;
        let bit: Bit = (accumulator < 1.0).into();
        if accumulator >= 1.0 {
            accumulator -= 1.0;
        }
        if iteration >= warm_up {
            let cost = estimator.prediction().to_fix_u32::<FinalProbability>()
                .estimate_cost(bit, log_lut);
            total_cost = total_cost.add(&cost.to_fix_i64::<Log2Q>());
            if bit.is_0() {
                zeros += 1;
            }
        }
        estimator.update(bit, lut);
    }
    let ones = measured - zeros;
    let real_probability = zeros as f64 / measured as f64;
    let ideal_cost = zeros as f64 * -real_probability.log2() +
        ones as f64 * -(1.0 - real_probability).log2();
    assert!(total_cost.as_f64() / ideal_cost - 1.0 < max_overhead,
            "{} {} {}", total_cost.as_f64(), ideal_cost, max_overhead);
}

fn check_fixed_speed_estimator_single(probability: f64,
                                      log_lut: &Log2Lut,
                                      max_overhead: f64) {
    assert!(probability > 0.0 && probability < 1.0);
    let mut estimator =
        FixedSpeedEstimator::new((probability * (1 << 16) as f64) as u16);
    let mut total_cost = Log2Q::new(0, LOG2_ACCURATE_BITS);
    let mut zeros = 0;
    let mut accumulator = probability;
    let warm_up = 2000;
    let measured = 5000;
    let total = warm_up + measured;
    for iteration in 0..total {
        accumulator += probability;
        let bit: Bit = (accumulator < 1.0).into();
        if accumulator >= 1.0 {
            accumulator -= 1.0;
        }
        if iteration >= warm_up {
            let cost = estimator.prediction().to_fix_u32::<FinalProbability>()
                .estimate_cost(bit, log_lut);
            total_cost = total_cost.add(&cost.to_fix_i64::<Log2Q>());
            if bit.is_0() {
                zeros += 1;
            }
        }
        estimator.update(bit);
    }
    let ones = measured - zeros;
    let real_probability = zeros as f64 / measured as f64;
    let ideal_cost = zeros as f64 * -real_probability.log2() +
        ones as f64 * -(1.0 - real_probability).log2();
    assert!(total_cost.as_f64() / ideal_cost - 1.0 < max_overhead,
            "{} {} {}, {} {} {}", total_cost.as_f64(), ideal_cost, max_overhead,
            probability, real_probability, estimator.prediction().as_f64());
}
