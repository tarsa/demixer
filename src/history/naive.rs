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
use bit::Bit;
use super::{
    HistorySource,
    ContextState,
    CollectedContextStates,
    updated_bit_history,
};
use super::window::{InputWindow, WindowIndex};

pub struct NaiveHistorySource {
    input: InputWindow,
    bit_index: i32,
    max_order: usize,
}

impl HistorySource for NaiveHistorySource {
    fn new(max_window_size: usize, max_order: usize) -> NaiveHistorySource {
        NaiveHistorySource {
            input: InputWindow::new(max_window_size, 0),
            bit_index: -1,
            max_order,
        }
    }

    fn start_new_byte(&mut self) {
        assert_eq!(self.bit_index, -1);
        self.bit_index = 7;
        assert_ne!(self.input.size(), self.input.max_size(),
                   "input window is filled up, but sliding is not implemented");
        self.input.advance_cursor();
    }

    fn gather_history_states(&self,
                             bit_histories: &mut CollectedContextStates) {
        for order in 0..(self.max_order + 1) {
            let mut last_occurrence_index_opt = None;
            let mut bit_history = 1;
            let stop_search_position =
                self.input.index_subtract(self.input.cursor(), order).raw();
            for scanned_index in 0..stop_search_position {
                let scanned_index = WindowIndex::new(scanned_index);
                let prefix_equal = self.input.compare_for_equal_prefix(
                    scanned_index,
                    self.input.index_subtract(self.input.cursor(), order),
                    self.bit_index as usize, order,
                );
                if prefix_equal {
                    last_occurrence_index_opt = Some(scanned_index);
                    let next_bit = self.input.get_bit(
                        self.input.index_add(scanned_index, order),
                        self.bit_index as usize);
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

    fn process_input_bit(&mut self, input_bit: Bit) {
        self.input.set_bit_at_cursor(input_bit, self.bit_index as usize);
        self.bit_index -= 1;
    }
}
