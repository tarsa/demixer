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
use PRINT_DEBUG;
use fixed_point::{FixedPoint, FixU32, fix_u32, FixI64};
use fixed_point::types::{FractOnlyU32, StretchedProbD, StretchedProbQ};
use lut::squash::SquashLut;

pub struct ApmWeightingLut {
    squashed_interval_stops: Vec<FractOnlyU32>,
    shifts_by_interval: Vec<u8>,
    extra_factor_by_interval: Vec<FractOnlyU32>,
    stretched_scale_down_bits: u8,
}

impl ApmWeightingLut {
    pub fn new(stretched_scale_down_bits: u8, squash_lut: &SquashLut) -> Self {
        if PRINT_DEBUG {
            println!("apm weighting lut, scale down bits: {}",
                     stretched_scale_down_bits);
        }
        let interval_stops_count =
            StretchedProbD::interval_stops_count(stretched_scale_down_bits);
        let mut squashed_stops = Vec::with_capacity(interval_stops_count);
        let mut shifts_by_interval = Vec::with_capacity(interval_stops_count);
        let mut extra_factor_by_interval =
            Vec::with_capacity(interval_stops_count);
        let offset = interval_stops_count as i32 / 2;
        for interval_stop_index in 0..interval_stops_count {
            let stretched_unscaled =
                (interval_stop_index as i32 - offset) as i64;
            let stretched_prob: StretchedProbD = StretchedProbQ::new(
                stretched_unscaled << (40 + stretched_scale_down_bits), 40)
                .clamped().to_fix_i32();
            let squashed_prob = squash_lut.squash(stretched_prob);
            squashed_stops.push(squashed_prob);
            if PRINT_DEBUG {
                println!("interval stop index: {:3}, squashed stop raw: {:12}",
                         interval_stop_index, squashed_prob.raw());
            }
        }
        let intervals_count = interval_stops_count - 1;
        let mut intervals_length_sum = FractOnlyU32::ZERO;
        for interval_index in 0..intervals_count {
            let interval_length = squashed_stops[interval_index + 1].sub(
                &squashed_stops[interval_index]);
            let shift = interval_length.raw().leading_zeros() as u8 - 1;
            let shifted = FractOnlyU32::new(interval_length.raw() << shift, 31);
            let extra_factor = Self::extra_factor(shifted);
            let scaled = shifted.add(&fix_u32::mul(&shifted, &extra_factor));
            if PRINT_DEBUG {
                println!("int_idx: {:3}, int_len {:9} : {:8.6}, \
                          shift: {:2}, shifted: {:8.6} \
                          extra factor: {:8.6}, scaled total: {:12.10}",
                         interval_index,
                         interval_length.raw(), interval_length.as_f64(),
                         shift, shifted.as_f64(),
                         extra_factor.as_f64(), scaled.as_f64());
            }
            shifts_by_interval.push(shift);
            extra_factor_by_interval.push(extra_factor);
            intervals_length_sum = intervals_length_sum.add(&interval_length);
        }
        assert_eq!(squashed_stops.len(), interval_stops_count);
        assert_eq!(shifts_by_interval.len(), intervals_count);
        assert_eq!(extra_factor_by_interval.len(), intervals_count);
        for index in 0..interval_stops_count {
            assert_eq!(squashed_stops[index],
                       squashed_stops[interval_stops_count - index - 1].flip());
        }
        for index in 0..intervals_count {
            assert_eq!(shifts_by_interval[index],
                       shifts_by_interval[intervals_count - index - 1]);
            assert_eq!(extra_factor_by_interval[index],
                       extra_factor_by_interval[intervals_count - index - 1]);
        }
        if PRINT_DEBUG {
            println!("interval lengths sum: {}", intervals_length_sum.as_f64());
        }
        ApmWeightingLut {
            squashed_interval_stops: squashed_stops,
            shifts_by_interval,
            extra_factor_by_interval,
            stretched_scale_down_bits,
        }
    }

    fn extra_factor(shifted_interval_length: FractOnlyU32) -> FractOnlyU32 {
        if shifted_interval_length == FractOnlyU32::ZERO {
            return FractOnlyU32::ZERO;
        }
        let remainder = FractOnlyU32::ONE_UNSAFE.sub(&shifted_interval_length);
        let mut low = FractOnlyU32::ZERO;
        let mut high = FractOnlyU32::ONE_UNSAFE.sub(&FractOnlyU32::ulp());
        while high.sub(&low) > FractOnlyU32::ulp() {
            let diff_raw = high.raw() - low.raw();
            let mid = low.add(&FractOnlyU32::new_unchecked(diff_raw / 2));
            let current: FractOnlyU32 =
                fix_u32::mul(&mid, &shifted_interval_length);
            if current < remainder {
                low = mid;
            } else {
                high = mid;
            }
        }
        low
    }

    pub fn squashed_interval_stops(&self) -> &[FractOnlyU32] {
        self.squashed_interval_stops.as_slice()
    }

    pub fn shifts_by_interval(&self) -> &[u8] {
        self.shifts_by_interval.as_slice()
    }

    pub fn extra_factor_by_interval(&self) -> &[FractOnlyU32] {
        self.extra_factor_by_interval.as_slice()
    }

    pub fn stretched_scale_down_bits(&self) -> u8 {
        self.stretched_scale_down_bits
    }
}
