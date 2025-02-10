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

use demixer::PRINT_DEBUG;
use demixer::bit::Bit;
use demixer::estimators::decelerating::DeceleratingEstimator;
use demixer::fixed_point::{FixedPoint, FixI32, FixU32};
use demixer::fixed_point::types::{FractOnlyU32, FractOnlyU64, StretchedProbD};
use demixer::lut::LookUpTables;
use demixer::lut::apm::ApmWeightingLut;
use demixer::lut::estimator::DeceleratingEstimatorRates;
use demixer::lut::squash::SquashLut;
use demixer::lut::stretch::StretchLut;
use demixer::mixing::apm::AdaptiveProbabilityMap;
use demixer::predictor::post_process::apm_factor_indexes;
use demixer::random::MersenneTwister;

#[test]
fn sanity_checks() {
    let stretch_lut = StretchLut::new(false);
    let squash_lut = SquashLut::new(&stretch_lut, false);
    for bits in 0..=LookUpTables::APM_LUTS_MAX_STRETCHED_SCALE_DOWN_BITS {
        AdaptiveProbabilityMap::new(0, bits, &squash_lut);
    }
}

#[test]
fn initial_mapping_is_close_to_identity() {
    let stretched_scale_down_bits: u8 = 0;
    let stretch_lut = StretchLut::new(false);
    let squash_lut = SquashLut::new(&stretch_lut, false);
    let apm_lut = ApmWeightingLut::new(stretched_scale_down_bits, &squash_lut);
    let full_count = 100_000;
    let step = 1f64 / full_count as f64;
    let print_interval = 100;
    let mut apm = AdaptiveProbabilityMap::new(
        full_count, stretched_scale_down_bits, &squash_lut);
    let mut previous = squash_lut.squash(StretchedProbD::MIN);
    for index in 1..full_count {
        let remainder = index % print_interval;
        let print = (remainder == 1 || remainder == print_interval - 1) &&
            PRINT_DEBUG;
        let input = (index as f64) / (full_count as f64);
        let input_sq = FractOnlyU32::new((FractOnlyU32::ONE_UNSAFE.raw() as f64
            * input) as u32, 31);
        let input_st = stretch_lut.stretch(input_sq);
        let refined_sq = apm.refine(index - 1, input_sq, input_st, &apm_lut);
        let refined_st = stretch_lut.stretch(refined_sq);
        let diff_sq = refined_sq.as_f64() - input_sq.as_f64();
        let diff_st = refined_st.as_f64() - input_st.as_f64();
        if print {
            println!("input sq: {:8.6}, refined sq: {:8.6}, diff sq: {:9.6}, \
                      input st: {:10.6}, refined st: {:10.6}, diff st: {:9.6}",
                     input_sq.as_f64(), refined_sq.as_f64(), diff_sq,
                     input_st.as_f64(), refined_st.as_f64(), diff_st);
        }
        assert!(diff_sq < 0.000_000_000_5, "{} {}", diff_sq, index);
        assert!(diff_st < 0.000_000_5, "{} {}", diff_st, index);
        assert!(refined_sq.raw() >= previous.raw(),
                "prev: {}, curr: {}", previous.as_f64(), refined_sq.as_f64());
        assert!(refined_sq.sub(&previous).as_f64() < step * 1.001,
                "step: {:12.10}, diff: {:12.10}",
                step, refined_sq.sub(&previous).as_f64());
        apm.update_predictions(index - 1, Bit::One, 1, 1, false);
        previous = refined_sq;
    }
}

#[test]
fn mapping_converges_to_real_probability() {
    test_single_apm_context(1)
}

#[test]
fn mapping_is_stable_with_many_models() {
    test_single_apm_context(100)
}

fn test_single_apm_context(counters_num: usize) {
    let stretched_scale_down_bits: u8 = 0;
    let stretch_lut = StretchLut::new(false);
    let squash_lut = SquashLut::new(&stretch_lut, false);
    let estimator_rates_lut = DeceleratingEstimatorRates::make_default();
    let apm_lut = ApmWeightingLut::new(stretched_scale_down_bits, &squash_lut);
    let fixed_weight = false;
    for &(orig_model_freq, orig_real_freq, max_overhead) in [
        (0.5, 0.5, 0.0005), (0.2, 0.7, 0.005),
        (0.01, 0.1, 0.02), (0.005, 0.01, 0.1), (0.001, 0.01, 0.1),
    ].iter() {
        assert!(orig_model_freq <= 0.5);
        for &(model_freq, real_freq, flipped) in [
            (orig_model_freq, orig_real_freq, false),
            (1.0 - orig_model_freq, 1.0 - orig_real_freq, true),
        ].iter() {
            let mut model_prng = {
                let seed = FractOnlyU64::from_f64(orig_model_freq).raw();
                MersenneTwister::new_by_scalar_seed(seed)
            };
            let mut real_prng = {
                let seed = FractOnlyU64::from_f64(orig_real_freq).raw();
                MersenneTwister::new_by_scalar_seed(seed)
            };
            if PRINT_DEBUG {
                println!("model freq: {:8.6}, real freq: {:8.6}, flipped: {}, \
                          max_overhead: {:6.4}",
                         model_freq, real_freq, flipped, max_overhead);
            }
            for round in 0..10 {
                if PRINT_DEBUG { println!("round #{}", round); }
                let mut counters: Vec<DeceleratingEstimator> = (0..counters_num)
                    .map(|_| DeceleratingEstimator::new()).collect();
                let mut apm = AdaptiveProbabilityMap::new(
                    1, stretched_scale_down_bits, &squash_lut);
                let warmup = 100_000;
                let non_warmup = 100_000;
                let mut total_cost = 0f64;
                for i in 0..warmup + non_warmup {
                    let counter = &mut counters[i % counters_num];
                    let counter_sq = counter.prediction();
                    let counter_st = stretch_lut.stretch(counter_sq);
                    let refined_sq =
                        apm.refine(0, counter_sq, counter_st, &apm_lut);
                    let model_input_bit: Bit = ((model_prng.next_real2() >=
                        orig_model_freq) ^ flipped).into();
                    counter.update(model_input_bit, &estimator_rates_lut);
                    let real_input_bit: Bit = ((real_prng.next_real2() >=
                        orig_real_freq) ^ flipped).into();
                    apm.update_predictions(0, real_input_bit,
                                           10, 10, fixed_weight);
                    if i >= warmup {
                        match real_input_bit {
                            Bit::Zero =>
                                total_cost -= refined_sq.as_f64().log2(),
                            Bit::One =>
                                total_cost -= (1.0 - refined_sq.as_f64()).log2(),
                        }
                    }
                }
                let average_cost = total_cost / non_warmup as f64;
                let ideal_average_cost = {
                    -(1.0f64 - real_freq).log2() * (1.0 - real_freq)
                        + -real_freq.log2() * real_freq
                };
                let overhead =
                    (average_cost - ideal_average_cost) / ideal_average_cost;
                if PRINT_DEBUG {
                    println!("average cost:       {}", average_cost);
                    println!("ideal average cost: {}", ideal_average_cost);
                    println!("overhead:           {}", overhead);
                    println!("counter 0:          {}",
                             counters[0].prediction().as_f64());
                    println!();
                }
                assert!(overhead < max_overhead);
            }
        }
    }
}

#[test]
fn mapping_is_symmetric() {
    let stretched_scale_down_bits: u8 =
        LookUpTables::APM_LUTS_MAX_STRETCHED_SCALE_DOWN_BITS;
    let stretch_lut = StretchLut::new(false);
    let squash_lut = SquashLut::new(&stretch_lut, false);
    let apm_lut = ApmWeightingLut::new(stretched_scale_down_bits, &squash_lut);
    for &(model_freq, real_freq) in [
        (0.01, 0.1), (0.2, 0.7), (0.5, 0.5), (0.001, 0.01)
    ].iter() {
        assert!(model_freq <= 0.5);
        let mut prng = MersenneTwister::default();
        if PRINT_DEBUG {
            println!("model freq: {:8.6}, real freq: {:8.6}",
                     model_freq, real_freq);
        }
        let mut a_apm = AdaptiveProbabilityMap::new(
            1, stretched_scale_down_bits, &squash_lut);
        let mut b_apm = AdaptiveProbabilityMap::new(
            1, stretched_scale_down_bits, &squash_lut);
        let a_model_sq = FractOnlyU32::from_f64(model_freq);
        let b_model_sq = a_model_sq.flip();
        let a_model_st = stretch_lut.stretch(a_model_sq);
        let b_model_st = stretch_lut.stretch(b_model_sq);
        assert_eq!(a_model_sq.raw() + b_model_sq.raw(), 1 << 31);
        assert_eq!(a_model_st, b_model_st.neg());
        for round in 0..1000 {
            let a_refined_sq =
                a_apm.refine(0, a_model_sq, a_model_st, &apm_lut);
            let b_refined_sq =
                b_apm.refine(0, b_model_sq, b_model_st, &apm_lut);
            let asymmetry_fract = ( // aka rounding error
                a_refined_sq.raw() as i64 - b_refined_sq.flip().raw() as i64
            ).abs();
            assert!(asymmetry_fract <= 2, "diff: {}, round: {}",
                    asymmetry_fract, round);
            let a_input_bit: Bit = (prng.next_real2() >= real_freq).into();
            let b_input_bit: Bit = !a_input_bit;
            a_apm.update_predictions(0, a_input_bit, 10, 10, false);
            b_apm.update_predictions(0, b_input_bit, 10, 10, false);
        }
    }
}

#[test]
fn mapping_factors_indexes_are_correct() {
    for &(input64, scale_down_bits, factors_indexes) in &[
        (1.0, 0, (1, 2)), (-1.0, 0, (2, 1)),
        (0.5, 0, (0, 1)), (-0.5, 0, (1, 0)),
        (1.0, 1, (0, 2)), (-1.0, 1, (2, 0)),
        (1.0, 2, (0, 4)), (-1.0, 2, (4, 0)),
        (8.0, 0, (8, 9)), (-8.0, 0, (9, 8)),
        (8.0, 1, (8, 10)), (-8.0, 1, (10, 8)),
        (8.0, 2, (8, 12)), (-8.0, 2, (12, 8)),
        (9.9, 0, (9, 10)), (-9.9, 0, (10, 9)),
        (9.9, 1, (8, 10)), (-9.9, 1, (10, 8)),
        (9.9, 2, (8, 12)), (-9.9, 2, (12, 8)),
        (12.0, 0, (11, 12)), (-12.0, 0, (12, 11)),
        (12.0, 1, (10, 12)), (-12.0, 1, (12, 10)),
        (12.0, 2, (8, 12)), (-12.0, 2, (12, 8)),
    ] {
        let input = StretchedProbD::from_f64(input64);
        assert_eq!(apm_factor_indexes(input.to_interval_index(scale_down_bits),
                                      scale_down_bits),
                   factors_indexes, "{} {}", input64, scale_down_bits);
    }
}
