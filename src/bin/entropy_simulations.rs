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
use demixer::coding::FinalProbability;
use demixer::estimators::decelerating::DeceleratingEstimator;
use demixer::fixed_point::{FixedPoint, FixI32, FixU32, FixI64};
use demixer::fixed_point::types::Log2Q;
use demixer::lut::estimator::DeceleratingEstimatorLut;
use demixer::lut::log2::{LOG2_ACCURATE_BITS, Log2Lut};

fn main() {
    let log_lut = Log2Lut::new();
    for x in -16..16 + 1 {
        let power = (-x as f64) / 2.0;
        println!("power: {}", power);
        let probability = 1f64 / (1f64 + power.exp());
        for &single_run_length in [1, 2, 3, 5, 8, 13, 21, 34, 55, 89,
            100, 200, 300, 1000, 2000].iter() {
            println!("single run length = {}", single_run_length);
            for &(factor, addend) in [(1, 2), (1, 3), (2, 2), (2, 3)].iter() {
                print!("LUT: factor = {}, addend = {} ", factor, addend);
                let lut = DeceleratingEstimatorLut::make(factor, addend);
                check_decelerating_estimator_single(probability, &lut, &log_lut,
                                                    single_run_length);
            }
        }
        println!();
    }
}

fn check_decelerating_estimator_single(probability: f64,
                                       lut: &DeceleratingEstimatorLut,
                                       log_lut: &Log2Lut,
                                       single_run_length: u32) {
    assert!(probability > 0.0 && probability < 1.0);
    let mut estimator = DeceleratingEstimator::new();
    let mut total_cost = Log2Q::new(0, LOG2_ACCURATE_BITS);
    let mut zeros = 0;
    let mut accumulator = probability;
    let total_predictions = 10_000;
    for i in 0..total_predictions {
        if i % single_run_length == 0 {
            estimator = DeceleratingEstimator::new();
        }
        accumulator += probability;
        let bit: Bit = (accumulator < 1.0).into();
        if accumulator >= 1.0 {
            accumulator -= 1.0;
        }
        let cost = estimator.prediction().to_fix_u32::<FinalProbability>()
            .estimate_cost(bit, log_lut);
        total_cost = total_cost.add(&cost.to_fix_i64::<Log2Q>());
        if bit.is_0() {
            zeros += 1;
        }
        estimator.update(bit, lut);
    }
    let ones = total_predictions - zeros;
    let real_probability = zeros as f64 / total_predictions as f64;
    let ideal_cost = zeros as f64 * -real_probability.log2() +
        ones as f64 * -(1.0 - real_probability).log2();
    println!("probability: {:.7}, total cost: {:14.7}, ideal cost: {:14.7}, \
              overhead: {:11.7}, ratio: {:11.7}, zeros: {:5}, ones: {:5}",
             probability, total_cost.as_f64(), ideal_cost,
             total_cost.as_f64() / ideal_cost - 1.0,
             total_predictions as f64 / total_cost.as_f64(), zeros, ones);
}
