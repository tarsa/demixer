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
use demixer::lut::LookUpTables;

#[test]
fn compare_for_one_byte_input() {
    let luts = LookUpTables::new();
    for max_order in 0..MAX_ORDER + 1 {
        for byte in 0..10 {
            compare_for_input(&[byte], max_order, true, &luts);
        }
        for byte in 100..105 {
            compare_for_input(&[byte], max_order, true, &luts);
        }
        for byte in 200..205 {
            compare_for_input(&[byte], max_order, true, &luts);
        }
        compare_for_input(&[255], max_order, true, &luts);
    }
}

#[test]
fn compare_for_repeated_byte_input() {
    let luts = LookUpTables::new();
    for max_order in 0..MAX_ORDER + 1 {
        compare_for_input(&[0xb5; 1], max_order, true, &luts);
        compare_for_input(&[' ' as u8; 2], max_order, true, &luts);
        compare_for_input(&['a' as u8; 5], max_order, true, &luts);
    }
}

#[test]
fn compare_for_two_symbols_sequences() {
    let luts = LookUpTables::new();
    let symbols_pairs: &[(u8, u8)] =
        &[(0, 255), ('b' as u8, 'a' as u8), (215, 15), (31, 32)];
    for &(sym_0, sym_1) in symbols_pairs.iter() {
        // regularly interrupted runs
        {
            for &interruption_period in [2, 3, 4, 7, 10].iter() {
                let mut input0 = vec![sym_0; interruption_period];
                input0.push(sym_1);
                let mut input1 = input0.clone();
                while input1.len() < 20 {
                    input1.append(&mut input0.clone());
                }
                for &max_order in [0, 1, 2, 3, 7, 20, 40, MAX_ORDER].iter() {
                    compare_for_input(&input1, max_order, true, &luts);
                }
            }
        }
        // fibonacci word
        {
            let mut word0 = vec![sym_0];
            let mut word1 = vec![sym_0, sym_1];
            while word1.len() < 20 {
                let old_word1 = word1.clone();
                word1.append(&mut word0);
                word0 = old_word1;
            }
            for &max_order in [0, 1, 2, 3, 7, 20, 40, MAX_ORDER].iter() {
                compare_for_input(&word1, max_order, true, &luts);
            }
        }
    }
}

#[test]
fn compare_for_multi_symbol_sequences() {
    let luts = LookUpTables::new();
    for &starting_symbol in [0 as u8, 'a' as u8].iter() {
        let mut word = vec![starting_symbol];
        let mut next_symbol = starting_symbol + 1;
        while word.len() < 20 {
            let mut clone = word.clone();
            word.append(&mut clone);
            word.push(next_symbol);
            next_symbol += 1;
        }
        for &max_order in [0, 1, 2, 3, 7, 20, 40, MAX_ORDER].iter() {
            compare_for_input(&word, max_order, true, &luts);
        }
    }
}

#[test]
fn compare_for_repeated_byte_borders() {
    let luts = LookUpTables::new();
    let border_and_middle_starter_symbols: &[(u8, u8)] =
        &[(0, 128), ('z' as u8, 'a' as u8), (215, 15), (31, 32)];
    for &(border_sym, middle_sym) in border_and_middle_starter_symbols.iter() {
        let mut middle = vec![middle_sym];
        let mut next_symbol: u8 = middle_sym + 1;
        while middle.len() < 10 {
            let mut clone = middle.clone();
            middle.append(&mut clone);
            middle.push(next_symbol);
            next_symbol += 1;
        }
        for &max_order in [0, 1, 2, 3, 7, 20].iter() {
            for &left_border_length in [1, 2, 3, 7].iter() {
                for &right_border_length in [1, 2, 3, 7].iter() {
                    let mut word = vec![border_sym; left_border_length];
                    word.append(&mut middle.clone());
                    word.append(&mut vec![border_sym; right_border_length]);
                    compare_for_input(&word, max_order, true, &luts);
                }
            }
        }
        assert!(middle.len() >= 10);
    }
}

#[test]
fn compare_for_repeated_pattern_borders() {
    let luts = LookUpTables::new();
    let border_and_middle_starter_symbols: &[(u8, u8, u8)] =
        &[(0, 255, 128), ('z' as u8, 'v' as u8, 'a' as u8), (31, 32, 215)];
    for &(border_sym_0, border_sym_1, middle_sym)
        in border_and_middle_starter_symbols.iter() {
        let mut middle = vec![middle_sym];
        let mut next_symbol: u8 = middle_sym + 1;
        while middle.len() < 10 {
            let mut clone = middle.clone();
            middle.append(&mut clone);
            middle.push(next_symbol);
            next_symbol += 1;
        }
        for &interruption_period in [1, 2, 3].iter() {
            let mut border = vec![border_sym_0; interruption_period];
            border.push(border_sym_1);
            while border.len() < 10 {
                let mut clone = border.clone();
                border.append(&mut clone);
            }
            for &max_order in [0, 1, 2, 3, 7, 20].iter() {
                for &left_border_length in [1, 2, 3].iter() {
                    for &right_border_length in [1, 2, 3].iter() {
                        let mut word = Vec::new();
                        word.extend_from_slice(&border[..left_border_length]);
                        word.append(&mut middle.clone());
                        word.extend_from_slice(&border[..right_border_length]);
                        compare_for_input(&word, max_order, true, &luts);
                    }
                }
            }
            assert!(border.len() >= 10);
        }
        assert!(middle.len() >= 10);
    }
}
