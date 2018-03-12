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
use history::{bytes_differ_on, compare_for_equal_prefix};

pub struct InputWindow {
    pub buffer: Vec<u8>,
    pub start: usize,
    pub cursor: usize,
    pub size: usize,
    pub max_size: usize,
}

impl InputWindow {
    pub fn index_add(&self, index: usize, to_add: usize) -> usize {
        let mut result = index + to_add;
        if result >= self.max_size {
            assert_eq!(self.buffer.len(), self.max_size);
            result -= self.max_size;
        }
        result
    }

    pub fn index_increment(&self, index: usize) -> usize {
        self.index_add(index, 1)
    }

    pub fn index_subtract(&self, index: usize, to_subtract: usize) -> usize {
        if index >= to_subtract {
            index - to_subtract
        } else {
            assert_eq!(self.buffer.len(), self.max_size);
            index + self.max_size - to_subtract
        }
    }

    pub fn index_decrement(&self, index: usize) -> usize {
        self.index_subtract(index, 1)
    }

    pub fn index_is_smaller(&self, idx1: usize, idx2: usize) -> bool {
        let fst = if idx1 < self.start { idx1 + self.max_size } else { idx1 };
        let snd = if idx2 < self.start { idx2 + self.max_size } else { idx2 };
        fst < snd
    }

    pub fn index_is_smaller_or_equal(&self, idx_1: usize, idx_2: usize) -> bool {
        !self.index_is_smaller(idx_2, idx_1)
    }

    pub fn compare_for_equal_prefix(&self, starting_index_first: usize,
                                    starting_index_second: usize,
                                    bit_index: usize,
                                    full_byte_length: usize) -> bool {
        if starting_index_first.max(starting_index_second) + full_byte_length
            < self.max_size {
            compare_for_equal_prefix(&self.buffer, starting_index_first,
                                     starting_index_second, bit_index,
                                     full_byte_length)
        } else {
            let mut index_first = starting_index_first;
            let mut index_second = starting_index_second;
            let mut equal = true;
            for _ in 0..full_byte_length {
                equal &= self.buffer[index_first] == self.buffer[index_second];
                if !equal { break; }
                index_first = self.index_increment(index_first);
                index_second = self.index_increment(index_second);
            }
            equal &= compare_for_equal_prefix(&self.buffer, index_first,
                                              index_second, bit_index, 0);
            equal
        }
    }

    pub fn bytes_differ_on(&self, first_byte_index: usize,
                           second_byte_index: usize, bit_index: usize) -> bool {
        bytes_differ_on(first_byte_index, second_byte_index, bit_index,
                        &self.buffer)
    }
}
