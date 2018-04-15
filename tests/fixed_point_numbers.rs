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

use demixer::fixed_point::{FixedPoint, FixU32, FixU64};
use demixer::fixed_point::types::Log2D;
use demixer::lut::log2::Log2Lut;

struct FixU32F17(u32);

impl FixedPoint for FixU32F17 {
    type Raw = u32;
    fn raw(&self) -> u32 { self.0 }
    fn new_unchecked(raw: u32) -> Self { FixU32F17(raw) }

    const FRACTIONAL_BITS: u8 = 17;
}

struct FixU64F33(u64);

impl FixedPoint for FixU64F33 {
    type Raw = u64;
    fn raw(&self) -> u64 { self.0 }
    fn new_unchecked(raw: u64) -> Self { FixU64F33(raw) }

    const FRACTIONAL_BITS: u8 = 33;
}


#[test]
fn log2_is_correct() {
    let lut = Log2Lut::new();
    test_log2(0.00003, &lut);
    test_log2(0.34523, &lut);
    test_log2(1.43646, &lut);
    test_log2(2.46723, &lut);
    test_log2(3463.64, &lut);
    test_log2(30000.0, &lut);
}

fn test_log2(input: f64, lut: &Log2Lut) {
    test_u32_log2(input, lut);
    test_u64_log2(input, lut);
}

fn test_u32_log2(input: f64, lut: &Log2Lut) {
    assert!(input > 0.0);
    let input = {
        let scaled = (input * (1u32 << 17) as f64).round();
        assert_eq!(scaled, scaled as u32 as f64);
        FixU32F17::new(scaled as u32, 17)
    };
    let log2 = input.as_f64().log2();
    let expected_log2 = {
        let scaled = (log2 * (1 << 11) as f64).round() as i32;
        Log2D::new(scaled, 11)
    };
    let actual_log2 = input.log2(lut);
    assert!((actual_log2.raw() - expected_log2.raw()).abs() <= 1);
}

fn test_u64_log2(input: f64, lut: &Log2Lut) {
    assert!(input > 0.0);
    let input = {
        let scaled = (input * (1u64 << 33) as f64).round();
        assert_eq!(scaled, scaled as u64 as f64);
        FixU64F33::new(scaled as u64, 33)
    };
    let log2 = input.as_f64().log2();
    let expected_log2 = {
        let scaled = (log2 * (1 << 11) as f64).round() as i32;
        Log2D::new(scaled, 11)
    };
    let actual_log2 = input.log2(lut);
    assert!((actual_log2.raw() - expected_log2.raw()).abs() <= 1);
}
