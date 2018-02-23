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
use history::{
    HistorySource,
    ContextState,
    CollectedContextStates,
    updated_bit_history, get_bit, compare_for_equal_prefix,
};

pub struct NaiveHistorySource {
    input: Vec<u8>,
    input_cursor: usize,
    bit_index: usize,
    max_order: usize,
}

impl HistorySource for NaiveHistorySource {
    fn new(max_window_size: usize, max_order: usize) -> NaiveHistorySource {
        NaiveHistorySource {
            input: Vec::with_capacity(max_window_size),
            input_cursor: 0,
            bit_index: 7,
            max_order,
        }
    }

    fn start_new_byte(&mut self) {
        assert_eq!(self.bit_index, 7);
        assert_eq!(self.input_cursor, self.input.len());
        assert_ne!(self.input.len(), self.input.capacity(),
                   "input window is filled up, but sliding is not implemented");
        self.input.push(0);
    }

    fn gather_history_states(&self,
                             bit_histories: &mut CollectedContextStates) {
        for order in 0..(self.max_order + 1) {
            let mut last_occurrence_index_opt = None;
            let mut bit_history = 1;
            for scanned_index in 0..(self.input_cursor - order) {
                let prefix_equal = compare_for_equal_prefix(
                    &self.input, scanned_index, self.input_cursor - order,
                    self.bit_index, order,
                );
                if prefix_equal {
                    last_occurrence_index_opt = Some(scanned_index);
                    let next_bit = get_bit(self.input[scanned_index + order],
                                           self.bit_index);
                    bit_history = updated_bit_history(bit_history, next_bit);
                }
            }
            assert_eq!(last_occurrence_index_opt == None, bit_history == 1);
            if let Some(last_occurrence_index) = last_occurrence_index_opt {
                bit_histories.items.push(
                    ContextState { last_occurrence_index, bit_history });
            } else {
                break;
            }
        }
    }

    fn process_input_bit(&mut self, input_bit: bool) {
        self.input[self.input_cursor] |= (input_bit as u8) << self.bit_index;
        if self.bit_index > 0 {
            self.bit_index -= 1;
        } else {
            self.bit_index = 7;
            self.input_cursor += 1;
        }
    }
}
