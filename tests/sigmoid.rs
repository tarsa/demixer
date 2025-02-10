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
use demixer::fixed_point::{FixedPoint, FixI32};
use demixer::fixed_point::types::{FractOnlyU32, StretchedProbD};
use demixer::lut::squash::SquashLut;
use demixer::lut::stretch::StretchLut;
use demixer::util::interpolate_f64;

#[test]
fn stretch_can_handle_extreme_values() {
    let lut = StretchLut::new(PRINT_DEBUG);
    for &input_raw in [1, (1u32 << 31) - 1].iter() {
        let input = FractOnlyU32::new(input_raw, 31);
        let input64 = input.as_f64();
        let expected = (input64 / (1.0 - input64)).ln();
        let actual = lut.stretch(input).as_f64();
        if PRINT_DEBUG {
            println!("expected = {:12.8}, actual = {:12.8}", expected, actual);
        }
        assert!(expected.abs() > actual.abs());
    }
}

#[test]
fn stretch_lut_is_correct() {
    let lut = StretchLut::new(PRINT_DEBUG);
    for &(start, stop, accuracy, intervals) in [
        (0.2500, 0.5000, 0.000_001, 1u32 << StretchLut::IN_LEVEL_INDEX_BITS),
        (0.7500, 0.5000, 0.000_001, 1u32 << StretchLut::IN_LEVEL_INDEX_BITS),
        (0.2500, 0.5000, 0.000_001, 100),
        (0.7500, 0.5000, 0.000_001, 100),
        (0.4900, 0.5100, 0.000_001, 1000),
        (0.0100, 0.1000, 0.000_001, 9999),
        (0.9900, 0.9000, 0.000_001, 9999),
        (0.0001, 0.0002, 0.000_700, 1500),
        (0.9999, 0.9998, 0.000_700, 1500),
    ].iter() {
        for input64 in interpolate_f64(start, stop, intervals) {
            let input = FractOnlyU32::from_f64(input64);
            let expected = (input64 / (1.0 - input64)).ln();
            let actual = lut.stretch(input).as_f64();
            if PRINT_DEBUG {
                println!("expected = {:12.8}, actual = {:12.8}",
                         expected, actual);
            }
            assert!((expected - actual).abs() < accuracy, "diff = {}",
                    (expected - actual).abs());
        }
    }
}

#[test]
fn squash_can_handle_extreme_values() {
    let stretch_lut = StretchLut::new(PRINT_DEBUG);
    let lut = SquashLut::new(&stretch_lut, PRINT_DEBUG);
    for &multiplier in [-1, 1].iter() {
        let bits = StretchedProbD::FRACTIONAL_BITS;
        let input_raw = multiplier * StretchedProbD::ABSOLUTE_LIMIT << bits;
        let input = StretchedProbD::new(input_raw, bits);
        let input64 = input.as_f64();
        let expected = 1.0 / (1.0 + (-input64).exp());
        let actual = lut.squash(input).as_f64();
        if PRINT_DEBUG {
            println!("input = {:12.8}, expected = {:12.8}, actual = {:12.8}",
                     input64, expected, actual);
        }
        assert!(actual > 0.0 && actual < 1.0);
        assert!((expected - actual).abs() < 0.000_002,
                "{}", (expected - actual).abs());
    }
}

#[test]
fn squash_lut_is_correct() {
    let stretch_lut = StretchLut::new(false);
    let lut = SquashLut::new(&stretch_lut, PRINT_DEBUG);
    for &(start, stop, accuracy, intervals) in [
        (-01.0, 001.0, 0.000_001, 100),
        (-11.0, -09.0, 0.000_001, 100),
        (011.0, 009.0, 0.000_001, 100),
        (-08.0, -05.0, 0.000_000_1, 9999),
        (008.0, 005.0, 0.000_000_1, 9999),
    ].iter() {
        for input64 in interpolate_f64(start, stop, intervals) {
            let input = StretchedProbD::from_f64(input64);
            let expected = 1.0 / (1.0 + (-input64).exp());
            let actual = lut.squash(input).as_f64();
            if PRINT_DEBUG {
                println!("input = {:12.8}, expected = {:12.8}, \
                          actual = {:12.8}, diff = {:12.8}",
                         input64, expected, actual, (expected - actual).abs());
            }
            assert!((expected - actual).abs() < accuracy, "diff = {}",
                    (expected - actual).abs());
        }
    }
}

#[test]
fn squash_and_stretch_composed_are_close_to_identity() {
    let stretch_lut = StretchLut::new(false);
    let squash_lut = SquashLut::new(&stretch_lut, false);
    for &(start, stop, accuracy, intervals) in [
        (0.2500, 0.5000, 0.000_001, 1u32 << StretchLut::IN_LEVEL_INDEX_BITS),
        (0.7500, 0.5000, 0.000_001, 1u32 << StretchLut::IN_LEVEL_INDEX_BITS),
        (0.2500, 0.5000, 0.000_001, 100),
        (0.7500, 0.5000, 0.000_001, 100),
        (0.4900, 0.5100, 0.000_001, 1000),
        (0.0100, 0.1000, 0.000_001, 9999),
        (0.9900, 0.9000, 0.000_001, 9999),
        (0.0001, 0.0002, 0.000_000_1, 1500),
        (0.9999, 0.9998, 0.000_000_1, 1500),
    ].iter() {
        for input64 in interpolate_f64(start, stop, intervals) {
            let input = FractOnlyU32::from_f64(input64);
            let stretched_direct = (input64 / (1.0 - input64)).ln();
            let squashed_direct = 1.0 / (1.0 + (-stretched_direct).exp());
            let stretched = stretch_lut.stretch(input);
            let squashed = squash_lut.squash(stretched).as_f64();
            if PRINT_DEBUG {
                println!("input = {:12.8}, \
                          output lut = {:12.8}, diff = {:12.8}, \
                          output direct = {:12.8}, diff = {:12.8}",
                         input64, squashed, (squashed - input64).abs(),
                         squashed_direct, (squashed_direct - input64).abs());
            }
            assert!((squashed - input64).abs() < accuracy, "diff = {}",
                    (squashed - input64).abs());
        }
    }
}

#[test]
fn stretch_is_symmetric() {
    let lut = StretchLut::new(PRINT_DEBUG);
    for &(start, stop, intervals) in [
        (0.2500, 0.5000, 1u32 << StretchLut::IN_LEVEL_INDEX_BITS),
        (0.2500, 0.5000, 100),
        (0.4900, 0.5000, 1000),
        (0.0100, 0.1000, 9999),
        (0.0001, 0.0002, 1500),
    ].iter() {
        for input64 in interpolate_f64(start, stop, intervals) {
            let a_sq = FractOnlyU32::from_f64(input64);
            let b_sq = a_sq.flip();
            let a_st = lut.stretch(a_sq);
            let b_st = lut.stretch(b_sq);
            assert_eq!(a_st, b_st.neg());
        }
    }
}

#[test]
fn squash_is_symmetric() {
    let stretch_lut = StretchLut::new(false);
    let lut = SquashLut::new(&stretch_lut, PRINT_DEBUG);
    for &(start, stop, intervals) in [
        (-01.0, 001.0, 100),
        (-11.0, -09.0, 100),
        (011.0, 009.0, 100),
        (-08.0, -05.0, 9999),
        (008.0, 005.0, 9999),
    ].iter() {
        for input64 in interpolate_f64(start, stop, intervals) {
            let a_st = StretchedProbD::from_f64(input64);
            let b_st = a_st.neg();
            let a_sq = lut.squash(a_st);
            let b_sq = lut.squash(b_st);
            assert_eq!(a_sq, b_sq.flip());
        }
    }
}
