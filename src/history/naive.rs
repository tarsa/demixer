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
use lut::LookUpTables;
use super::{
    ContextState, CollectedContextStates, HistorySource,
};
use super::window::{InputWindow, WindowIndex};
use super::tree::node::CostTrackers;

pub struct NaiveHistorySource<'a> {
    luts: &'a LookUpTables,
    input: InputWindow,
    bit_index: i32,
    max_order: usize,
}

impl<'a> HistorySource<'a> for NaiveHistorySource<'a> {
    fn new(max_window_size: usize, max_order: usize, luts: &'a LookUpTables)
           -> NaiveHistorySource {
        NaiveHistorySource {
            luts,
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
            let mut last_context_state_opt: Option<ContextState> = None;
            let stop_search_position =
                self.input.index_subtract(self.input.cursor(), order).raw();
            for scanned_index in 0..stop_search_position {
                let scanned_index = WindowIndex::new(scanned_index);
                let current_occurrence_index =
                    self.input.index_subtract(self.input.cursor(), order);
                let prefix_equal = self.input.compare_for_equal_prefix(
                    scanned_index, current_occurrence_index,
                    self.bit_index as usize, order,
                );
                if prefix_equal {
                    let bit_in_context = self.input.get_bit(
                        self.input.index_add(scanned_index, order),
                        self.bit_index as usize);
                    let new_context_state = {
                        let current_occurrence_distance = self.input.index_diff(
                            current_occurrence_index, scanned_index);
                        if let Some(context_state) = last_context_state_opt {
                            let cost_trackers_opt = Some(CostTrackers::DEFAULT)
                                .filter(|_| context_state.is_for_node());
                            context_state.next_state(
                                current_occurrence_distance, bit_in_context,
                                cost_trackers_opt, self.luts)
                        } else {
                            ContextState::starting_state(
                                current_occurrence_distance, bit_in_context)
                        }
                    };
                    last_context_state_opt = Some(new_context_state);
                }
            }
            if let Some(context_state) = last_context_state_opt {
                bit_histories.items.push(context_state);
            } else {
                break;
            }
        }
    }

    fn process_input_bit(&mut self, input_bit: Bit,
                         new_cost_trackers: &[CostTrackers]) {
        assert!(new_cost_trackers.iter().all(|c| *c == CostTrackers::DEFAULT),
                "cost tracking is unsupported in this history source");
        self.input.set_bit_at_cursor(input_bit, self.bit_index as usize);
        self.bit_index -= 1;
    }
}
