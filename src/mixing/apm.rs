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
use DO_CHECKS;
use bit::Bit;
use fixed_point::{FixedPoint, FixU32, fix_u32, FixI64};
use fixed_point::types::{FractOnlyU32, StretchedProbD, StretchedProbQ};
use lut::apm::ApmWeightingLut;
use lut::squash::SquashLut;


pub const SHARED_ENDPOINTS: bool = true;

pub struct AdaptiveProbabilityMap {
    mappings: Vec<FractOnlyU32>,
    stretched_scale_down_bits: u8,
    contexts_number: usize,
    saved_left_context_index: i32,
    saved_left_weight: FractOnlyU32,
}

impl AdaptiveProbabilityMap {
    /// PAQ8 default:
    ///
    /// stretched_fract_index_bits = 1
    pub fn new(contexts_number: usize, stretched_scale_down_bits: u8,
               squash_lut: &SquashLut) -> Self {
        let single_mapping_size =
            if SHARED_ENDPOINTS {
                StretchedProbD::interval_stops_count(stretched_scale_down_bits)
            } else {
                StretchedProbD::intervals_count(stretched_scale_down_bits) * 2
            };
        let mappings_size = contexts_number * single_mapping_size;
        let mut mappings = Vec::with_capacity(mappings_size);
        if contexts_number > 0 {
            if SHARED_ENDPOINTS {
                let offset = single_mapping_size / 2;
                for interval_stop in 0..single_mapping_size {
                    let stretched_unscaled = (interval_stop - offset) as i64;
                    let stretched_prob: StretchedProbD = StretchedProbQ::new(
                        stretched_unscaled << (40 + stretched_scale_down_bits),
                        40).clamped().to_fix_i32();
                    let input_prob = squash_lut.squash(stretched_prob);
                    mappings.push(input_prob);
                }
                for i in 0..=offset as usize {
                    assert_eq!(mappings[offset + i].flip().raw(),
                               mappings[offset - i].raw());
                }
            } else {
                let intervals_count = StretchedProbD::intervals_count(
                    stretched_scale_down_bits);
                assert_eq!(intervals_count & 1, 0);
                for distance_from_0 in (0..(intervals_count / 2) as i64).rev() {
                    let left_prob_st: StretchedProbD = StretchedProbQ::new(
                        (distance_from_0 + 1) <<
                            (40 + stretched_scale_down_bits), 40)
                        .neg().clamped().to_fix_i32();
                    let right_prob_st: StretchedProbD = StretchedProbQ::new(
                        distance_from_0 << (40 + stretched_scale_down_bits), 40)
                        .neg().clamped().to_fix_i32();
                    mappings.push(squash_lut.squash(left_prob_st));
                    mappings.push(squash_lut.squash(right_prob_st));
                }
                for distance_from_0 in 0..(intervals_count / 2) as i64 {
                    let left_prob_st: StretchedProbD = StretchedProbQ::new(
                        distance_from_0 << (40 + stretched_scale_down_bits), 40)
                        .clamped().to_fix_i32();
                    let right_prob_st: StretchedProbD = StretchedProbQ::new(
                        (distance_from_0 + 1) <<
                            (40 + stretched_scale_down_bits), 40)
                        .clamped().to_fix_i32();
                    mappings.push(squash_lut.squash(left_prob_st));
                    mappings.push(squash_lut.squash(right_prob_st));
                }
                for distance in 0..intervals_count {
                    assert_eq!(mappings[intervals_count + distance]
                                   .flip().raw(),
                               mappings[intervals_count - distance - 1].raw());
                }
            };
        }
        for _ in 1..contexts_number {
            for input in 0..single_mapping_size {
                let reused_value = mappings[input];
                mappings.push(reused_value);
            }
        }
        assert_eq!(mappings.len(), mappings_size);
        AdaptiveProbabilityMap {
            mappings,
            stretched_scale_down_bits,
            contexts_number,
            saved_left_context_index: -1,
            saved_left_weight: FractOnlyU32::ZERO,
        }
    }

    pub fn refine(&mut self, context: usize,
                  input_sq: FractOnlyU32, input_st: StretchedProbD,
                  apm_lut: &ApmWeightingLut) -> FractOnlyU32 {
        assert_eq!(self.stretched_scale_down_bits,
                   apm_lut.stretched_scale_down_bits());
        assert_eq!(self.saved_left_context_index, -1);
        let scale_down_bits = self.stretched_scale_down_bits;
        let stops_count = StretchedProbD::interval_stops_count(scale_down_bits);
        let last_interval_stop = stops_count - 1;
        let input_sq = input_sq
            .min(apm_lut.squashed_interval_stops()[last_interval_stop])
            .max(apm_lut.squashed_interval_stops()[0]);
        let index_left = input_st.to_interval_index(scale_down_bits);
        if DO_CHECKS {
            assert!(index_left + 1 < stops_count,
                    "{} {}", index_left, stops_count);
        }
        let index_left =
            if input_sq < apm_lut.squashed_interval_stops()[index_left] {
                assert_ne!(index_left, 0);
                index_left - 1
            } else if index_left < stops_count && input_sq > apm_lut
                .squashed_interval_stops()[index_left + 1] {
                index_left + 1
            } else {
                index_left
            };
        let interval_index = index_left;
        assert!(input_sq >= apm_lut.squashed_interval_stops()[index_left]);
        assert!(input_sq <= apm_lut.squashed_interval_stops()[index_left + 1]);
        let weight_right =
            input_sq.sub(&apm_lut.squashed_interval_stops()[index_left]);
        let weight_right = FractOnlyU32::new(
            weight_right.raw() << apm_lut.shifts_by_interval()[interval_index],
            31);
        let weight_right = weight_right.add(&fix_u32::mul(
            &weight_right, &apm_lut.extra_factor_by_interval()[interval_index])
        );
        let weight_left = match weight_right {
            FractOnlyU32::ZERO => FractOnlyU32::ONE_UNSAFE,
            other => FractOnlyU32::ONE_UNSAFE.sub(&other),
        };
        let single_mapping_size =
            if SHARED_ENDPOINTS {
                StretchedProbD::interval_stops_count(scale_down_bits)
            } else {
                StretchedProbD::intervals_count(scale_down_bits) * 2
            };
        let mapping_start = context * single_mapping_size;
        let index_left =
            if SHARED_ENDPOINTS { index_left } else { index_left * 2 };
        let left_bound = self.mappings[mapping_start + index_left];
        let right_bound = self.mappings[mapping_start + index_left + 1];
        self.saved_left_context_index = index_left as i32;
        self.saved_left_weight = weight_left;
        let result_right: FractOnlyU32 = fix_u32::mul(
            &right_bound, &weight_right);
        let result_left: FractOnlyU32 = fix_u32::mul(
            &left_bound, &weight_left);
        result_left.add(&result_right)
    }

    pub fn update_predictions(&mut self, context: usize, input_bit: Bit,
                              left_log_rate: u8, right_log_rate: u8,
                              fixed_weight: bool) {
        assert!(self.saved_left_context_index >= 0);
        assert!(context < self.contexts_number);
        let scale_down_bits = self.stretched_scale_down_bits;
        let single_mapping_size =
            if SHARED_ENDPOINTS {
                StretchedProbD::interval_stops_count(scale_down_bits)
            } else {
                StretchedProbD::intervals_count(scale_down_bits) * 2
            };
        let base_index = context * single_mapping_size;
        let left_context_index = self.saved_left_context_index as usize;
        let weight_left =
            if fixed_weight {
                FractOnlyU32::HALF
            } else {
                self.saved_left_weight
            };
        let weight_right = FractOnlyU32::ONE_UNSAFE.sub(&weight_left);
        self.update_single_entry(input_bit, base_index + left_context_index,
                                 weight_left, left_log_rate);
        self.update_single_entry(input_bit, base_index + left_context_index + 1,
                                 weight_right, right_log_rate);
        self.saved_left_context_index = -1;
    }

    fn update_single_entry(&mut self, input_bit: Bit, entry_index: usize,
                           weight: FractOnlyU32, log_rate: u8) {
        assert!(log_rate > 0 && log_rate < 30);
        let entry = self.mappings[entry_index];
        match input_bit {
            Bit::Zero => {
                let error = FractOnlyU32::ONE_UNSAFE.sub(&entry);
                let correction = fix_u32::mul(
                    &weight, &FractOnlyU32::new(error.raw() >> log_rate, 31));
                self.mappings[entry_index] = entry.add(&correction);
            }
            Bit::One => {
                let error = entry;
                let correction = fix_u32::mul(
                    &weight, &FractOnlyU32::new(error.raw() >> log_rate, 31));
                self.mappings[entry_index] = entry.sub(&correction);
            }
        };
    }
}
