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
use fixed_point::{FixedPoint, FixI32, FixU32, fix_u32};
use fixed_point::types::{FractOnlyU32, StretchedProbD};
use lut::apm::ApmWeightingLut;
use lut::squash::SquashLut;


pub struct AdaptiveProbabilityMap {
    mappings: Vec<FractOnlyU32>,
    stretched_fract_index_bits: u8,
    contexts_number: usize,
    saved_left_context_index: i32,
    saved_left_weight: FractOnlyU32,
}

impl AdaptiveProbabilityMap {
    /// PAQ8 default:
    ///
    /// stretched_fract_index_bits = 1
    pub fn new(contexts_number: usize, stretched_fract_index_bits: u8,
               squash_lut: &SquashLut) -> Self {
        let single_mapping_size =
            StretchedProbD::interval_stops_count(stretched_fract_index_bits);
        let offset = single_mapping_size / 2;
        let mappings_size = contexts_number * single_mapping_size as usize;
        let mut mappings = Vec::with_capacity(mappings_size);
        if contexts_number > 0 {
            for interval_stop in 0..single_mapping_size {
                let stretched_unscaled = (interval_stop - offset) as i32;
                let stretched_prob = StretchedProbD::new(
                    stretched_unscaled << (21 - stretched_fract_index_bits),
                    21);
                let input_prob = squash_lut.squash(stretched_prob);
                mappings.push(input_prob);
            }
            for i in 0..=offset as usize {
                assert_eq!(mappings[offset as usize + i].flip().raw(),
                           mappings[offset as usize - i].raw());
            }
        }
        for _ in 1..contexts_number {
            for input in 0..single_mapping_size {
                let reused_value = mappings[input as usize];
                mappings.push(reused_value);
            }
        }
        assert_eq!(mappings.len(), mappings_size);
        AdaptiveProbabilityMap {
            mappings,
            stretched_fract_index_bits,
            contexts_number,
            saved_left_context_index: -1,
            saved_left_weight: FractOnlyU32::ZERO,
        }
    }

    pub fn refine(&mut self, context: usize,
                  input_sq: FractOnlyU32, input_st: StretchedProbD,
                  apm_lut: &ApmWeightingLut) -> FractOnlyU32 {
        assert_eq!(self.stretched_fract_index_bits,
                   apm_lut.stretched_fract_index_bits());
        assert_eq!(self.saved_left_context_index, -1);
        let fract_index_bits = self.stretched_fract_index_bits;
        let stops_count =
            StretchedProbD::interval_stops_count(fract_index_bits);
        let last_interval_stop = stops_count - 1;
        let input_sq = input_sq
            .min(apm_lut.squashed_interval_stops()[last_interval_stop as usize])
            .max(apm_lut.squashed_interval_stops()[0]);
        let input_st = match input_st {
            StretchedProbD::MIN =>
                StretchedProbD::MIN.add(&StretchedProbD::ulp()),
            StretchedProbD::MAX =>
                StretchedProbD::MAX.sub(&StretchedProbD::ulp()),
            other => other,
        };
        let index_scale = StretchedProbD::FRACTIONAL_BITS - fract_index_bits;
        let offset = stops_count / 2;
        let index_left = (input_st.raw() >> index_scale) + offset;
        assert!(index_left >= 0 && index_left + 1 < stops_count);
        let index_left = index_left as usize;
        let index_left =
            if input_sq < apm_lut.squashed_interval_stops()[index_left] {
                assert_ne!(index_left, 0);
                index_left - 1
            } else { index_left };
        let index_right = index_left + 1;
        let interval_index = index_left;
        assert!(input_sq >= apm_lut.squashed_interval_stops()[index_left]);
        assert!(input_sq <= apm_lut.squashed_interval_stops()[index_right]);
        let mapping_start = context * stops_count as usize;
        let left_bound = self.mappings[mapping_start + index_left];
        let right_bound = self.mappings[mapping_start + index_right];
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
        self.saved_left_context_index = index_left as i32;
        self.saved_left_weight = weight_left;
        let result_right: FractOnlyU32 = fix_u32::mul(
            &right_bound, &weight_right);
        let result_left: FractOnlyU32 = fix_u32::mul(
            &left_bound, &weight_left);
        result_left.add(&result_right)
    }

    pub fn update_predictions(&mut self, context: usize, input_bit: Bit,
                              log_rate: u8, fixed_weight: bool) {
        assert!(self.saved_left_context_index >= 0);
        assert!(context < self.contexts_number);
        let base_index = context * StretchedProbD::interval_stops_count(
            self.stretched_fract_index_bits) as usize;
        let left_context_index = self.saved_left_context_index as usize;
        let weight_left =
            if fixed_weight {
                FractOnlyU32::HALF
            } else {
                self.saved_left_weight
            };
        let weight_right = FractOnlyU32::ONE_UNSAFE.sub(&weight_left);
        self.update_single_entry(input_bit, weight_left,
                                 base_index + left_context_index, log_rate);
        self.update_single_entry(input_bit, weight_right,
                                 base_index + left_context_index + 1, log_rate);
        self.saved_left_context_index = -1;
    }

    fn update_single_entry(&mut self, input_bit: Bit, weight: FractOnlyU32,
                           entry_index: usize, log_rate: u8) {
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
