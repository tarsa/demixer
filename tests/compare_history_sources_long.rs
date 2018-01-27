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

mod compare_history_sources;

use compare_history_sources::compare_for_input;
use demixer::MAX_ORDER;

#[test]
#[cfg(not(feature = "long_tests"))]
#[ignore]
fn long_tests_skipped() {
    // silencing dead code and unused imports warnings
    compare_for_input(&[], 0, false);
    assert_eq!(MAX_ORDER, 0);
}

#[test]
#[cfg(feature = "long_tests")]
fn compare_for_one_byte_input() {
    for max_order in 0..MAX_ORDER + 1 {
        for byte in 0..255 {
            compare_for_input(&[byte], max_order, true);
        }
        compare_for_input(&[255], max_order, true);
    }
}

#[test]
#[cfg(feature = "long_tests")]
fn compare_for_repeated_byte_input() {
    for max_order in 0..MAX_ORDER + 1 {
        compare_for_input(&[0xb5 + max_order as u8; 80], max_order, false);
    }
}

#[test]
#[cfg(feature = "long_tests")]
fn compare_for_two_symbols_sequences() {
    let symbols_pairs: &[(u8, u8)] =
        &[(0, 255), ('a' as u8, 'b' as u8), (15, 215), (31, 32)];
    for &(sym_0, sym_1) in symbols_pairs.iter() {
        // fibonacci word
        {
            let mut word0 = vec![sym_0];
            let mut word1 = vec![sym_0, sym_1];
            while word1.len() < 300 {
                let old_word1 = word1.clone();
                word1.append(&mut word0);
                word0 = old_word1;
            }
            for &max_order in [0, 1, 2, 3, 7, 20, 40, MAX_ORDER].iter() {
                compare_for_input(&word1, max_order, false);
            }
        }
    }
}

#[test]
#[cfg(feature = "long_tests")]
fn compare_for_multi_symbol_sequences() {
    for &starting_symbol in [0 as u8, 'a' as u8].iter() {
        let mut word = vec![starting_symbol];
        let mut next_symbol = starting_symbol + 1;
        while word.len() < 300 {
            let mut clone = word.clone();
            word.append(&mut clone);
            word.push(next_symbol);
            next_symbol += 1;
        }
        for &max_order in [0, 1, 2, 3, 7, 20, 40, MAX_ORDER].iter() {
            compare_for_input(&word, max_order, false);
        }
    }
}