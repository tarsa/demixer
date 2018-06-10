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
use fixed_point::{FixedPoint, FixI32, FixU32, fix_i32, fix_u32};
use fixed_point::types::{
    NoFractI32, NoFractU32, FractOnlyI32, FractOnlyU32, StretchedProbD,
};
use super::log2::one_plus_log2_restricted;

pub struct StretchLut([[StretchedProbD; 1usize <<
    StretchLut::IN_LEVEL_INDEX_BITS]; 1usize << StretchLut::LEVELS_INDEX_BITS]);

impl StretchLut {
    pub const LEVELS_INDEX_BITS: u8 = 3;
    pub const IN_LEVEL_INDEX_BITS: u8 = 9;
    const LAST_LEVEL: usize = (1 << Self::LEVELS_INDEX_BITS) - 1;
    const LAST_IN_LEVEL: usize = (1 << Self::IN_LEVEL_INDEX_BITS) - 1;

    pub fn minimum_accurately_mapped_input() -> FractOnlyU32 {
        FractOnlyU32::new(1u32 << (31 - 1 - Self::LAST_LEVEL as u8
            - Self::IN_LEVEL_INDEX_BITS), 31)
    }

    pub fn new(print: bool) -> StretchLut {
        let ln2 = FractOnlyI32::new(1488522236, 31);
        let filler = StretchedProbD::new_unchecked(<i32>::max_value());
        let mut stretch_lut = [[filler; 1 << Self::IN_LEVEL_INDEX_BITS]
            ; 1 << Self::LEVELS_INDEX_BITS];
        for level in (0usize..1 << Self::LEVELS_INDEX_BITS).rev() {
            let is_last_level = level == Self::LAST_LEVEL;
            let level_start: usize = if is_last_level { 1 } else { 0 };
            let level_start_fract =
                if is_last_level {
                    FractOnlyU32::new_unchecked(0)
                } else {
                    FractOnlyU32::new(1 << (31 - 2 - level as u32), 31)
                };
            let level_step_fract =
                if is_last_level {
                    FractOnlyU32::new(1 << (31 - 1 - Self::LAST_LEVEL as u8
                        - Self::IN_LEVEL_INDEX_BITS), 31)
                } else {
                    FractOnlyU32::new(1 << (31 - 2 - level as u8
                        - Self::IN_LEVEL_INDEX_BITS), 31)
                };
            if is_last_level {
                assert_eq!(level_start_fract.add(&level_step_fract),
                           Self::minimum_accurately_mapped_input());
            }
            for in_level_index in level_start..1 << Self::IN_LEVEL_INDEX_BITS {
                let input = level_start_fract.add(&fix_u32::mul(
                    &level_step_fract,
                    &NoFractU32::new(in_level_index as u32, 0)));
                let log2_direct = Self::log2(input);
                let flipped = input.flip();
                let log2_flipped = Self::log2(flipped);
                let log2 = log2_direct.sub(&log2_flipped);
                let ln: StretchedProbD = fix_i32::mul(&log2, &ln2);
                let actual = ln.as_f64();
                let expected = (input.as_f64() / (1.0 - input.as_f64())).ln();
                if print {
                    println!("level = {:1}, in level = {:3}, input = {:11.8}, \
                              expected = {:12.8}, actual = {:12.8}",
                             level, in_level_index, input.as_f64(),
                             expected, actual);
                }
                assert!((expected - actual).abs() < 0.000_001);
                stretch_lut[level][in_level_index] = ln;
            }
        }
        StretchLut(stretch_lut)
    }

    fn log2(fract: FractOnlyU32) -> StretchHelper {
        let fract = fract.raw();
        assert_ne!(fract, 0);
        let leading_zeros = fract.leading_zeros();
        assert!(leading_zeros > 0);
        let scaled = FractOnlyU32::new(fract << leading_zeros - 1, 31);
        let pre_log2 = one_plus_log2_restricted(scaled);
        let pre_log2 = FractOnlyI32::new(pre_log2.raw() as i32, 31);
        let pre_log2: StretchHelper = pre_log2.to_fix_i32();
        let excess = 1 + leading_zeros as i32 -
            (32 - FractOnlyU32::FRACTIONAL_BITS as i32);
        let excess: StretchHelper = NoFractI32::new(excess, 0).to_fix_i32();
        pre_log2.sub(&excess)
    }

    /// Inverse of squash. Return d = ln(p/(1-p)). Also called 'logit'.
    pub fn stretch(&self, input: FractOnlyU32) -> StretchedProbD {
        let half = FractOnlyU32::HALF;
        if input == half {
            return StretchedProbD::new_unchecked(0);
        }
        let flip = input > half;
        let input = {
            let flipped = input.flip();
            if flip { flipped } else { input }
        };
        let input = input.raw();
        assert_ne!(input, 0);
        let leading_zeros = input.leading_zeros();
        let level = Self::LAST_LEVEL.min(leading_zeros as usize - 2);
        let shift =
            if level == Self::LAST_LEVEL { level + 2 } else { level + 3 };
        let in_level_index_left = (input << shift) >>
            (FractOnlyU32::TOTAL_BITS - Self::IN_LEVEL_INDEX_BITS);
        let left_bound =
            if in_level_index_left != 0 || level != Self::LAST_LEVEL {
                self.0[level][in_level_index_left as usize]
            } else {
                self.0[level][1]
            };
        let right_bound =
            if in_level_index_left != Self::LAST_IN_LEVEL as u32 {
                self.0[level][in_level_index_left as usize + 1]
            } else if level > 0 {
                self.0[level - 1][0]
            } else {
                StretchedProbD::new_unchecked(0)
            };
        let weight_right = FractOnlyI32::new((((input as u32) <<
            shift + Self::IN_LEVEL_INDEX_BITS as usize) >> 1) as i32, 31);
        let result_left: StretchedProbD = left_bound.sub(&fix_i32::mul(
            &left_bound, &weight_right));
        let result_right: StretchedProbD = fix_i32::mul(
            &right_bound, &weight_right);
        let result = result_left.add(&result_right);
        if flip { result.neg() } else { result }
    }
}

struct StretchHelper(i32);

impl FixedPoint for StretchHelper {
    type Raw = i32;
    fn raw(&self) -> Self::Raw { self.0 }
    fn new_unchecked(raw: Self::Raw) -> Self { StretchHelper(raw) }

    const FRACTIONAL_BITS: u8 = 25;
}
