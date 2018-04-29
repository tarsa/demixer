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
use fixed_point::{FixedPoint, FixI32, FixU32, fix_u32};
use fixed_point::types::{FractOnlyU32, StretchedProbD};
use super::stretch::StretchLut;

pub struct SquashLut([FractOnlyU32; SquashLut::ARRAY_SIZE]);

impl SquashLut {
    const USED_FRACTIONAL_BITS: u8 = 7;
    const ARRAY_SIZE: usize = (1i32 + StretchedProbD::ABSOLUTE_LIMIT * 2 *
        (1 << SquashLut::USED_FRACTIONAL_BITS)) as usize;

    pub fn new(stretch_lut: &StretchLut, print: bool) -> SquashLut {
        let mut squash_lut = [FractOnlyU32::new(0, 31); Self::ARRAY_SIZE];
        let mut max_error = 0f64;
        let max_stretched_prob = -stretch_lut.stretch(
            StretchLut::minimum_accurately_mapped_input()).as_f64();
        assert!(max_stretched_prob > 0.0);
        for index in 0..Self::ARRAY_SIZE {
            let stretched_prob = StretchedProbD::new(
                (index as i32 - Self::ARRAY_SIZE as i32 / 2) <<
                    (21 - Self::USED_FRACTIONAL_BITS), 21);
            let expected = 1f64 / (1f64 + stretched_prob.neg().as_f64().exp());
            let actual = Self::find_squashed_prob(stretched_prob, stretch_lut);
            let error = (expected - actual.as_f64()).abs();
            if stretched_prob.as_f64().abs() <= max_stretched_prob {
                max_error = max_error.max(error);
            }
            if print {
                println!("index = {:4}, stretched prob = {:9.5}, expected \
                          = {:12.9}, actual = {:12.9}, error = {:12.9}",
                         index, stretched_prob.as_f64(), expected,
                         actual.as_f64(), error);
            }
            squash_lut[index] = actual;
        }
        if print { println!("max error = {:12.9}", max_error); }
        SquashLut(squash_lut)
    }

    fn find_squashed_prob(stretched_prob: StretchedProbD,
                          stretch_lut: &StretchLut) -> FractOnlyU32 {
        let ulp = FractOnlyU32::ulp();
        let mut lower_bound = FractOnlyU32::ZERO.add(&ulp).raw();
        let mut upper_bound = FractOnlyU32::ONE_UNSAFE.sub(&ulp).raw();
        while upper_bound - lower_bound > 1 {
            let middle = FractOnlyU32::new_unchecked(
                lower_bound + (upper_bound - lower_bound) / 2);
            let stretched_middle = stretch_lut.stretch(middle);
            if stretched_middle > stretched_prob {
                upper_bound = middle.raw();
            } else if stretched_middle < stretched_prob {
                lower_bound = middle.raw();
            } else {
                return middle;
            }
        }
        assert_eq!(lower_bound + 1, upper_bound);
        let lower_fract = FractOnlyU32::new(lower_bound, 31);
        let upper_fract = FractOnlyU32::new(upper_bound, 31);
        let lower_stretched = stretch_lut.stretch(lower_fract);
        let upper_stretched = stretch_lut.stretch(upper_fract);
        if lower_stretched != upper_stretched {
            assert!(lower_stretched <= stretched_prob);
            assert!(upper_stretched >= stretched_prob);
            let lower_diff = stretched_prob.sub(&lower_stretched);
            let upper_diff = upper_stretched.sub(&stretched_prob);
            if lower_diff < upper_diff {
                lower_fract
            } else {
                upper_fract
            }
        } else {
            let mut result = FractOnlyU32::new_unchecked(lower_bound);
            let minimum_precise_value =
                StretchLut::minimum_accurately_mapped_input();
            result = result.max(minimum_precise_value);
            result = result.min(minimum_precise_value.flip());
            result
        }
    }

    /// Inverse of stretch. Return p = 1/(1+exp(-d)). Also called 'expit'.
    pub fn squash(&self, input: StretchedProbD) -> FractOnlyU32 {
        let fract_bits = Self::USED_FRACTIONAL_BITS;
        let shift = StretchedProbD::FRACTIONAL_BITS - fract_bits;
        let offset = StretchedProbD::ABSOLUTE_LIMIT << fract_bits;
        let index_left = (input.raw() >> shift) + offset;
        assert!(index_left >= 0);
        let index_left = index_left as usize;
        if index_left == Self::ARRAY_SIZE - 1 {
            return self.0[index_left];
        }
        let index_right = index_left + 1;
        let left_bound = self.0[index_left];
        let right_bound = self.0[index_right];
        let weight_bits = shift;
        let weight_right = (input.raw() as u32) & ((1 << weight_bits) - 1);
        let weight_right =
            FractOnlyU32::new(weight_right << (31 - weight_bits), 31);
        let result_left: FractOnlyU32 = left_bound.sub(&fix_u32::mul(
            &left_bound, &weight_right));
        let result_right: FractOnlyU32 = fix_u32::mul(
            &right_bound, &weight_right);
        result_left.add(&result_right)
    }
}
