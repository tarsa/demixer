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

#[derive(Debug, Eq, PartialEq)]
pub struct ContextState {
    // TODO wrap in WindowIndex
    pub first_occurrence_index: usize,
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
    fn new(input_size: usize, max_order: usize) -> Self;

    fn start_new_byte(&mut self);

    fn gather_history_states(
        &self, context_states: &mut CollectedContextStates);

    fn process_input_bit(&mut self, input_bit: bool);
}

fn updated_bit_history(bit_history: u32, next_bit: u8) -> u32 {
    ((bit_history << 1) & 2047) | (next_bit as u32) | (bit_history & 1024)
}

pub fn get_bit(byte: u8, bit_index: i32) -> u8 {
    (byte >> bit_index) & 1
}

fn bytes_differ_on(contents: &[u8], first_byte_index: usize,
                   second_byte_index: usize, bit_index: i32) -> bool {
    get_bit(contents[first_byte_index] ^ contents[second_byte_index],
            bit_index) == 1
}

fn compare_for_equal_prefix(contents: &[u8], starting_index_first: usize,
                            starting_index_second: usize, bit_index: i32,
                            full_byte_length: usize) -> bool {
    let mut equal = true;
    for position in 0..full_byte_length {
        equal &= contents[starting_index_first + position] ==
            contents[starting_index_second + position];
        if !equal { break };
    }
    let mut bit_position = 7;
    while equal && bit_position > bit_index {
        equal &= !bytes_differ_on(contents,
                                  starting_index_first + full_byte_length,
                                  starting_index_second + full_byte_length,
                                  bit_position);
        bit_position -= 1;
    }
    equal
}
