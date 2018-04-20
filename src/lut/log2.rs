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
use fixed_point::{FixedPoint, FixU32, fix_u32};
use fixed_point::types::FractOnlyU32;

pub struct Log2Lut([u8; 1usize << Log2Lut::INDEX_BITS]);

impl Log2Lut {
    pub const INDEX_BITS: u8 = 11;

    pub fn new() -> Log2Lut {
        let mut log2_lut = [<u8>::max_value(); 1 << Self::INDEX_BITS];
        for input in 1 << Self::INDEX_BITS..2 << Self::INDEX_BITS {
            let half_input = FractOnlyU32::new(
                ((input as u32) << 31 - Self::INDEX_BITS - 1) as u32, 31);
            let log2 = one_plus_log2_restricted(half_input);
            let log2_scaled = log2.raw() >> 31 - Self::INDEX_BITS - 1;
            let log2_scaled = (log2_scaled + 1) >> 1;
            let index = input - (1 << Self::INDEX_BITS);
            let diff = log2_scaled - index as u32;
            assert!(diff <= 176);
            log2_lut[index] = diff as u8;
        }
        Log2Lut(log2_lut)
    }

    /// Input [1.0, 2.0) scaled by Self::INDEX_BITS
    ///
    /// Output [0.0, 1.0) scaled by Self::INDEX_BITS
    pub fn log2_restricted(&self, input: u32) -> u32 {
        let fract = input - (1 << Self::INDEX_BITS);
        self.0[fract as usize] as u32 + fract
    }
}

/// Input [0.5, 1.0)
///
/// Output [0.0, 1.0) = log2(input) + 1
pub fn one_plus_log2_restricted(input: FractOnlyU32) -> FractOnlyU32 {
    assert!(input.within_bounds());
    let half = FractOnlyU32::new(1 << 30, 31);
    let mut log_raw = 0u32;
    let mut a_power = input;
    assert_eq!(a_power.trunc(), 0);
    assert!(a_power >= half);
    for _ in 0..FractOnlyU32::FRACTIONAL_BITS {
        log_raw <<= 1;
        a_power = fix_u32::mul(&a_power, &a_power);
        if a_power < half {
            a_power = FractOnlyU32::new(a_power.raw() * 2, 31);
        } else {
            log_raw |= 1;
        }
    }
    FractOnlyU32::new(log_raw, 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lut_is_correct() {
        let lut = Log2Lut::new();
        let bits = 11;
        let scale = 1 << bits;
        for (index, input) in (scale..scale * 2).enumerate() {
            let log2_fract = (input as f64).log2().fract();
            let output = (log2_fract * scale as f64).round() as u32 + scale;
            assert!(output >= input);
            let diff = output - input;
            assert!(diff <= 176);
            assert_eq!(diff as u8, lut.0[index]);
            assert_eq!(lut.log2_restricted(input), output - scale);
        }
    }

    #[test]
    fn binary_logarithm_computation_is_correct() {
        for input in 500..1000 {
            let input = input as f64 / 1000.0;
            let expected = input.log2() + 1.0;
            let actual = one_plus_log2_restricted(
                FractOnlyU32::new((((1u32 << 31) as f64) * input) as u32, 31)
            ).as_f64();
            assert!((actual - expected).abs() < 0.00001);
        }
    }
}
