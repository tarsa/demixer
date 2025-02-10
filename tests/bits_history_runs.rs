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

use demixer::bit::Bit;
use demixer::history::state::bits_runs::BitsRunsTracker;
use demixer::random::MersenneTwister;

#[test]
fn sanity_checks() {
    let minimal_bit_run = BitsRunsTracker::new();
    assert!(minimal_bit_run.is_single_bit_run());
    assert_eq!(minimal_bit_run.last_bit_run_length(), 0);
    assert_eq!(minimal_bit_run.opposite_bit_run_length(), 0);
    assert_eq!(minimal_bit_run.last_bit_previous_run_length(), 0);

    let single_zero_bit_run = BitsRunsTracker::for_bit_run(Bit::Zero, 123);
    assert!(single_zero_bit_run.is_single_bit_run());
    assert_eq!(single_zero_bit_run.last_bit_run_length(), 123);
    assert_eq!(single_zero_bit_run.opposite_bit_run_length(), 0);
    assert_eq!(single_zero_bit_run.last_bit_previous_run_length(), 0);

    let single_one_bit_run = BitsRunsTracker::for_bit_run(Bit::Zero, 4352);
    assert!(single_one_bit_run.is_single_bit_run());
    assert_eq!(single_one_bit_run.last_bit_run_length(), 1000);
    assert_eq!(single_one_bit_run.opposite_bit_run_length(), 0);
    assert_eq!(single_one_bit_run.last_bit_previous_run_length(), 0);
}

#[test]
fn bit_runs_do_not_overflow() {
    let max_run_length = BitsRunsTracker::MAX_RUN_LENGTH;
    let mut bits_runs = BitsRunsTracker::new();

    bits_runs = push_bits(bits_runs, Bit::One, max_run_length + 10);
    assert_eq!(bits_runs.last_bit(), Bit::One);
    assert!(bits_runs.is_single_bit_run());
    assert_eq!(bits_runs.last_bit_run_length(), max_run_length);
    assert_eq!(bits_runs.opposite_bit_run_length(), 0);
    assert_eq!(bits_runs.last_bit_previous_run_length(), 0);

    bits_runs = push_bits(bits_runs, Bit::Zero, max_run_length + 10);
    assert_eq!(bits_runs.last_bit(), Bit::Zero);
    assert!(!bits_runs.is_single_bit_run());
    assert_eq!(bits_runs.last_bit_run_length(), max_run_length);
    assert_eq!(bits_runs.opposite_bit_run_length(), max_run_length);
    assert_eq!(bits_runs.last_bit_previous_run_length(), 0);

    bits_runs = push_bits(bits_runs, Bit::One, max_run_length + 10);
    assert_eq!(bits_runs.last_bit(), Bit::One);
    assert!(!bits_runs.is_single_bit_run());
    assert_eq!(bits_runs.last_bit_run_length(), max_run_length);
    assert_eq!(bits_runs.opposite_bit_run_length(), max_run_length);
    assert_eq!(bits_runs.last_bit_previous_run_length(), max_run_length);

    bits_runs = push_bits(bits_runs, Bit::Zero, max_run_length + 10);
    assert_eq!(bits_runs.last_bit(), Bit::Zero);
    assert!(!bits_runs.is_single_bit_run());
    assert_eq!(bits_runs.last_bit_run_length(), max_run_length);
    assert_eq!(bits_runs.opposite_bit_run_length(), max_run_length);
    assert_eq!(bits_runs.last_bit_previous_run_length(), max_run_length);
}

#[test]
fn bits_runs_are_properly_shifted_back_on_bit_change() {
    let max_run_length = BitsRunsTracker::MAX_RUN_LENGTH;
    let mut prng = MersenneTwister::default();
    let mut bits_runs = BitsRunsTracker::new();

    let mut last_bit_previous_run_length;
    let mut opposite_bit_run_length = 0;
    let mut last_bit_run_length = 0;

    let mut bit = Bit::Zero;

    for _ in 0..123 {
        let bits_run_length = (prng.next_int64() as u16) % (2u16 << 10);
        bits_runs = push_bits(bits_runs, bit, bits_run_length);

        assert_eq!(bits_runs.last_bit(), bit);
        bit = !bit;

        last_bit_previous_run_length = opposite_bit_run_length;
        opposite_bit_run_length = last_bit_run_length;
        last_bit_run_length = bits_run_length.min(max_run_length);
        assert_eq!(bits_runs.last_bit_run_length(), last_bit_run_length);
        assert_eq!(bits_runs.opposite_bit_run_length(),
                   opposite_bit_run_length);
        assert_eq!(bits_runs.last_bit_previous_run_length(),
                   last_bit_previous_run_length);
    }
}

fn push_bits(input_bits_runs: BitsRunsTracker, bit: Bit,
             run_length: u16) -> BitsRunsTracker {
    let mut bits_runs = input_bits_runs;
    for _ in 0..run_length {
        bits_runs = bits_runs.updated(bit);
    }
    bits_runs
}
