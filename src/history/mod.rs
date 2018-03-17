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
pub mod naive;
pub mod fat_map;
pub mod tree;
pub mod window;

use self::window::WindowIndex;

// TODO convert to enum with variants: ForNode, ForEdge
#[derive(Debug, Eq, PartialEq)]
pub struct ContextState {
    pub last_occurrence_index: WindowIndex,
    // TODO wrap in BitHistory
    pub bit_history: u32,
}

pub struct CollectedContextStates {
    items: Vec<ContextState>,
}

impl CollectedContextStates {
    pub fn new(max_order: usize) -> CollectedContextStates {
        CollectedContextStates {
            items: Vec::with_capacity(max_order + 1)
        }
    }

    pub fn items(&self) -> &[ContextState] {
        &self.items
    }

    pub fn push(&mut self, context_state: ContextState) {
        assert_ne!(self.items.len(), self.items.capacity());
        self.items.push(context_state);
    }

    pub fn reset(&mut self) {
        self.items.clear();
    }
}

pub trait HistorySource {
    fn new(max_window_size: usize, max_order: usize) -> Self;

    fn start_new_byte(&mut self);

    fn gather_history_states(
        &self, context_states: &mut CollectedContextStates);

    fn process_input_bit(&mut self, input_bit: bool);
}

fn make_bit_run_history(uncapped_length: usize, repeated_bit: bool) -> u32 {
    let length = 10.min(uncapped_length);
    let bit = repeated_bit as u32;
    (1 << length) | (((1 << length) - 1) * bit)
}

fn updated_bit_history(bit_history: u32, next_bit: bool) -> u32 {
    ((bit_history << 1) & 2047) | (next_bit as u32) | (bit_history & 1024)
}
