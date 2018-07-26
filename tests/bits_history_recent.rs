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
use demixer::history::state::recent_bits::RecentBitsHistory;

#[test]
fn history_states_are_properly_reported_as_valid() {
    assert!(!RecentBitsHistory::INVALID.is_valid());
    assert!(RecentBitsHistory::new_unchecked(1).is_valid());
}

#[test]
fn recent_bits_state_returns_proper_substates() {
    for &(full_state, histogram, last_7_bits) in [
        (0x0001, 0x00, 0x01),
        (0x0002, 0x01, 0x02),
        (0x0003, 0x10, 0x03),
        (0x0004, 0x02, 0x04),
        (0x0005, 0x11, 0x05),
        (0x0006, 0x11, 0x06),
        (0x0007, 0x20, 0x07),
        (0x4888, 0x3b, 0x88),
        (0x7777, 0xb3, 0xf7),
        (0x1eee, 0x93, 0xee),
        (0x1111, 0x39, 0x91),
        (0x8000, 0x0f, 0x80),
        (0xffff, 0xf0, 0xff),
    ].iter() {
        let state = RecentBitsHistory::new_unchecked(full_state);
        assert!(state.is_valid());
        assert_eq!(state.histogram(), histogram);
        assert_eq!(state.last_7_bits(), last_7_bits);
    }
}

#[test]
fn recent_bits_state_factory_return_proper_states() {
    for &(run_length, state_for_zeros, state_for_ones) in [
        (0, 0x0001, 0x0001), (1, 0x0002, 0x0003),
        (2, 0x0004, 0x0007), (3, 0x0008, 0x000f),
        (13, 0x2000, 0x3fff), (14, 0x4000, 0x7fff),
        (15, 0x8000, 0xffff), (16, 0x8000, 0xffff),
        (17, 0x8000, 0xffff), (30, 0x8000, 0xffff),
        (50, 0x8000, 0xffff), (99, 0x8000, 0xffff),
    ].iter() {
        assert_eq!(RecentBitsHistory::new_unchecked(state_for_zeros),
                   RecentBitsHistory::for_bit_run(Bit::Zero, run_length));
        assert_eq!(RecentBitsHistory::new_unchecked(state_for_ones),
                   RecentBitsHistory::for_bit_run(Bit::One, run_length));
    }
    for run_length in 0..100 {
        for &last_bit in [Bit::Zero, Bit::One].iter() {
            let previous_state =
                RecentBitsHistory::for_bit_run(!last_bit, run_length);
            let current_state =
                RecentBitsHistory::for_new_node(last_bit, run_length);
            assert_eq!(previous_state.updated(last_bit), current_state);
            assert!(previous_state.is_valid());
            assert!(current_state.is_valid());
        }
    }
}
