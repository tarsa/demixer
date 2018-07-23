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
use demixer::fixed_point::{FixedPoint, FixI32};
use demixer::fixed_point::types::FractOnlyU32;
use demixer::lut::estimator::DeceleratingEstimatorRates;
use demixer::lut::squash::SquashLut;
use demixer::lut::stretch::StretchLut;
use demixer::mixing::mixer::{
    Mixer, FixedSizeMixer, Mixer1, Mixer2, Mixer3, Mixer4, Mixer5, MixerN,
};
use demixer::random::MersenneTwister;

#[test]
fn mixers_pass_self_checks() {
    MixerN::new(30, 10, false);
    Mixer1::new(10, false);
    Mixer2::new(10, false);
    Mixer3::new(10, false);
    Mixer4::new(10, false);
    Mixer5::new(10, false);
}

#[test]
fn mixing_converges_to_real_probability() {
    mixer_converges_to_real_probability(|| MixerN::new(1, 10, false), &[
        (&[0.2], 0.5),
        (&[0.1], 0.01),
    ], 0.01);
    mixer_converges_to_real_probability(|| Mixer1::new(10, false), &[
        (&[0.2], 0.5),
        (&[0.1], 0.01),
    ], 0.01);
    mixer_converges_to_real_probability(|| Mixer2::new(10, false), &[
        (&[0.2, 0.9], 0.5),
        (&[0.1, 0.99], 0.01),
    ], 0.01);
    mixer_converges_to_real_probability(|| Mixer3::new(10, false), &[
        (&[0.2, 0.9, 0.7], 0.5),
        (&[0.1, 0.99, 0.9], 0.01),
    ], 0.01);
    mixer_converges_to_real_probability(|| Mixer4::new(10, false), &[
        (&[0.2, 0.9, 0.7, 0.1], 0.5),
        (&[0.1, 0.99, 0.9, 0.01], 0.01),
    ], 0.01);
    mixer_converges_to_real_probability(|| Mixer5::new(10, false), &[
        (&[0.2, 0.9, 0.7, 0.1, 0.5], 0.5),
        (&[0.1, 0.99, 0.9, 0.01, 0.5], 0.01),
    ], 0.01);
}

fn mixer_converges_to_real_probability<Mxr: Mixer>(
    make_mixer: fn() -> Mxr,
    input_freqs_with_real_freqs: &[(&[f64], f64)],
    max_overhead: f64,
) {
    let estimator_rates_lut = DeceleratingEstimatorRates::make_default();
    let stretch_lut = StretchLut::new(false);
    let squash_lut = SquashLut::new(&stretch_lut, false);
    let warmup_iterations = 100_000;
    let non_warmup_iterations = 100_000;
    for &(input_freqs, real_freq) in input_freqs_with_real_freqs.iter() {
        if PRINT_DEBUG {
            println!("max overhead: {:6.4}, \
                      real freq: {:8.6}, input freqs: {:?}",
                     max_overhead, real_freq, input_freqs);
        }
        for &flipped in [false, true].iter() {
            let mut prng = MersenneTwister::default();
            let mut mixer = make_mixer();

            let inputs_sq = input_freqs.iter()
                .map(|&freq| FractOnlyU32::from_f64(freq))
                .map(|fract| if flipped { fract.flip() } else { fract })
                .collect::<Vec<_>>();
            let inputs_st = inputs_sq.iter()
                .map(|&squashed| stretch_lut.stretch(squashed))
                .collect::<Vec<_>>();

            let mut total_cost = 0f64;
            for i in 0..warmup_iterations + non_warmup_iterations {
                for index in 0..input_freqs.len() {
                    mixer.set_input(index, inputs_sq[index], inputs_st[index]);
                }
                let (result_sq, _) = mixer.mix_all(&squash_lut);
                let input_bit: Bit =
                    (flipped ^ (prng.next_real2() >= real_freq)).into();
                mixer.update_and_reset(
                    input_bit, result_sq, 1000, &estimator_rates_lut);
                if i >= warmup_iterations {
                    match input_bit {
                        Bit::Zero =>
                            total_cost -= result_sq.as_f64().log2(),
                        Bit::One =>
                            total_cost -= (1.0 - result_sq.as_f64()).log2(),
                    }
                }
            }
            let average_cost = total_cost / non_warmup_iterations as f64;
            let ideal_average_cost = {
                -(1.0f64 - real_freq).log2() * (1.0 - real_freq)
                    + -real_freq.log2() * real_freq
            };
            let overhead =
                (average_cost - ideal_average_cost) / ideal_average_cost;
            if PRINT_DEBUG {
                println!("flipped:            {}", flipped);
                println!("average cost:       {}", average_cost);
                println!("ideal average cost: {}", ideal_average_cost);
                println!("overhead:           {}", overhead);
                print!("weights:            [{:8.6}", mixer.weight(0).as_f64());
                for index in 1..mixer.size() {
                    print!(", {:8.6}", mixer.weight(index).as_f64());
                }
                println!("]");
                println!();
            }
            assert!(overhead < max_overhead);
        }
    }
}

#[test]
fn mixing_is_symmetric() {
    mixer_is_symmetric(|| MixerN::new(1, 10, true), &[
        (&[0.2], 0.5), (&[0.1], 0.01),
    ]);
    mixer_is_symmetric(|| Mixer1::new(10, true), &[
        (&[0.2], 0.5), (&[0.1], 0.01),
    ]);
    mixer_is_symmetric(|| Mixer2::new(10, true), &[
        (&[0.2, 0.9], 0.5), (&[0.1, 0.99], 0.01),
    ]);
    mixer_is_symmetric(|| Mixer3::new(10, true), &[
        (&[0.2, 0.9, 0.7], 0.5), (&[0.1, 0.99, 0.9], 0.01),
    ]);
    mixer_is_symmetric(|| Mixer4::new(10, true), &[
        (&[0.2, 0.9, 0.7, 0.1], 0.5), (&[0.1, 0.99, 0.9, 0.01], 0.01),
    ]);
    mixer_is_symmetric(|| Mixer5::new(10, true), &[
        (&[0.2, 0.9, 0.7, 0.1, 0.5], 0.5), (&[0.1, 0.99, 0.9, 0.01, 0.5], 0.01),
    ]);
}

fn mixer_is_symmetric<Mxr: Mixer>(
    make_mixer: fn() -> Mxr,
    input_freqs_with_real_freqs: &[(&[f64], f64)],
) {
    let estimator_rates_lut = DeceleratingEstimatorRates::make_default();
    let stretch_lut = StretchLut::new(false);
    let squash_lut = SquashLut::new(&stretch_lut, false);
    for &(input_freqs, real_freq) in input_freqs_with_real_freqs.iter() {
        let mut prng = MersenneTwister::default();
        let mut mixer_a = make_mixer();
        let mut mixer_b = make_mixer();

        let inputs_sq = input_freqs.iter()
            .map(|&freq| FractOnlyU32::from_f64(freq)).collect::<Vec<_>>();
        let inputs_st = inputs_sq.iter()
            .map(|&squashed| stretch_lut.stretch(squashed)).collect::<Vec<_>>();

        for _ in 0..1000 {
            for index in 0..input_freqs.len() {
                mixer_a.set_input(index, inputs_sq[index], inputs_st[index]);
                mixer_b.set_input(index, inputs_sq[index], inputs_st[index]);
            }
            let (result_sq_a, result_st_a) = mixer_a.mix_all(&squash_lut);
            let (result_sq_b, result_st_b) = mixer_b.mix_all(&squash_lut);
            assert_eq!(result_sq_a, result_sq_b.flip());
            assert_eq!(result_st_a, result_st_b.neg());
            for index in 0..input_freqs.len() {
                assert_eq!(mixer_a.weight(index), mixer_b.weight(index).neg());
            }
            let a_input_bit: Bit = (prng.next_real2() >= real_freq).into();
            let b_input_bit: Bit = !a_input_bit;
            mixer_a.update_and_reset(
                a_input_bit, result_sq_a, 1000, &estimator_rates_lut);
            mixer_b.update_and_reset(
                b_input_bit, result_sq_b, 1000, &estimator_rates_lut);
        }
    }
}

#[test]
fn mixer_clamps_result_between_bounds() {
    let stretch_lut = StretchLut::new(false);
    let squash_lut = SquashLut::new(&stretch_lut, false);
    let d_estimator_rates_lut = DeceleratingEstimatorRates::make_default();
    let mut mixer = MixerN::new(1, 0, true);
    let input_sq = FractOnlyU32::from_f64(0.99);
    let input_st = stretch_lut.stretch(input_sq);
    let fixed_mix_result = FractOnlyU32::from_f64(0.6);
    for _ in 0..10_000 {
        mixer.set_input(0, input_sq, input_st);
        mixer.mix_all(&squash_lut);
        mixer.update_and_reset(Bit::Zero, fixed_mix_result, 10,
                               &d_estimator_rates_lut);
    }
}
