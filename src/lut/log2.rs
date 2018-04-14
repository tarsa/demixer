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
use ::fixed_point::*;

pub const LOG2_ACCURATE_BITS: u8 = 11;

pub struct Log2Lut([u8; 1 << LOG2_ACCURATE_BITS]);

impl Log2Lut {
    /// Input [1.0, 2.0) scaled by LOG2_ACCURATE_BITS
    ///
    /// Output [0.0, 1.0) scaled by LOG2_ACCURATE_BITS
    pub fn log2_restricted(&self, input: u32) -> u32 {
        let fract = input as usize - (1 << LOG2_ACCURATE_BITS);
        self.0[fract] as u32 + fract as u32
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Log2Type(u32);

impl FixedPoint for Log2Type {
    type Raw = u32;
    fn raw(&self) -> u32 { self.0 }
    fn new_unchecked(raw: u32) -> Self { Log2Type(raw) }

    const FRACTIONAL_BITS: u8 = 30;
}

pub fn make_log2_lut() -> Log2Lut {
    let two = Log2Type::new(2 << 30, 30);
    let mut log2_lut = [<u8>::max_value(); 1 << LOG2_ACCURATE_BITS];
    for index in 0usize..1 << LOG2_ACCURATE_BITS {
        let a = (1 << LOG2_ACCURATE_BITS) + index as u32;
        let mut log_raw = 0u32;
        let mut a_power =
            Log2Type::new((a as u32) << 30 - LOG2_ACCURATE_BITS, 30);
        for _ in 0..LOG2_ACCURATE_BITS + 1 {
            log_raw <<= 1;
            a_power = fix_u32::mul(&a_power, &a_power);
            if a_power >= two {
                log_raw |= 1;
                a_power = Log2Type::new(a_power.raw() / 2, 30);
            }
        }
        let log_scaled = (log_raw + 1) >> 1;
        let diff = log_scaled - index as u32;
        assert!(diff <= 176);
        log2_lut[index] = diff as u8;
    }
    Log2Lut(log2_lut)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lut_is_correct() {
        let lut = make_log2_lut();
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
}
