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
use std::{fmt, ops};

pub fn get_bit(byte: u8, bit_index: usize) -> bool {
    ((byte >> bit_index) & 1) == 1
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowIndex {
    value: usize
}

impl WindowIndex {
    pub fn new(value: usize) -> WindowIndex {
        WindowIndex { value }
    }

    pub fn raw(&self) -> usize {
        self.value
    }
}

impl fmt::Display for WindowIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

// TODO move to demixer::history
pub struct InputWindow {
    buffer: Vec<u8>,
    start: WindowIndex,
    cursor: WindowIndex,
    size: usize,
    max_size: usize,
}

impl InputWindow {
    pub fn new(max_window_size: usize, initial_shift: usize) -> InputWindow {
        assert!(max_window_size > 0);
        let mut buffer = Vec::with_capacity(max_window_size);
        buffer.resize(initial_shift, 0);
        assert_eq!(buffer.capacity(), max_window_size);
        assert_eq!(buffer.len(), initial_shift);
        let cursor = if initial_shift > 0 {
            initial_shift - 1
        } else {
            max_window_size - 1
        };
        InputWindow {
            buffer,
            start: WindowIndex::new(initial_shift),
            cursor: WindowIndex::new(cursor),
            size: 0,
            max_size: max_window_size,
        }
    }

    pub fn start(&self) -> WindowIndex {
        self.start
    }

    pub fn cursor(&self) -> WindowIndex {
        self.cursor
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn max_size(&self) -> usize {
        self.max_size
    }

    pub fn advance_start(&mut self) {
        assert_ne!(self.size, 0);
        self.buffer[self.start.raw()] = 0;
        self.start = self.index_increment(self.start);
        self.size -= 1;
        self.check_size_invariants();
    }

    pub fn advance_cursor(&mut self) {
        assert!(self.size < self.max_size);
        if self.buffer.len() == self.buffer.capacity() {
            self.cursor = self.index_increment(self.cursor);
            self.set_byte_at_cursor(0);
            assert_eq!(self.buffer.len(), self.max_size);
        } else {
            assert!(self.buffer.len() < self.max_size);
            self.buffer.push(0);
            self.cursor.value += 1;
            if self.cursor.value == self.max_size {
                self.cursor.value = 0
            };
        }
        self.size += 1;
        self.check_size_invariants();
    }

    fn check_size_invariants(&self) {
        assert!(self.size <= self.max_size);
        assert_eq!(self.buffer.capacity(), self.max_size);
        if self.size > 1 {
            assert_eq!(self.cursor, self.index_add(self.start, self.size - 1));
            assert_eq!(self.size - 1, self.index_diff(self.cursor, self.start));
        } else if self.size == 1 {
            assert_eq!(self.start, self.cursor);
        } else {
            assert_eq!(self.size, 0);
        }
    }

    pub fn index_add(&self, index: WindowIndex, to_add: usize) -> WindowIndex {
        let mut result = index.value + to_add;
        if result >= self.max_size {
            assert_eq!(self.buffer.len(), self.max_size);
            result -= self.max_size;
        }
        WindowIndex::new(result)
    }

    pub fn index_increment(&self, index: WindowIndex) -> WindowIndex {
        self.index_add(index, 1)
    }

    pub fn index_subtract(&self, index: WindowIndex,
                          to_subtract: usize) -> WindowIndex {
        let result =
            if index.value >= to_subtract {
                index.value - to_subtract
            } else {
                assert_eq!(self.buffer.len(), self.max_size);
                index.value + self.max_size - to_subtract
            };
        WindowIndex::new(result)
    }

    pub fn index_decrement(&self, index: WindowIndex) -> WindowIndex {
        self.index_subtract(index, 1)
    }

    pub fn index_diff(&self, target: WindowIndex,
                      source: WindowIndex) -> usize {
        self.index_subtract(target, source.value).value
    }

    pub fn index_is_smaller(&self, idx1: WindowIndex,
                            idx2: WindowIndex) -> bool {
        let idx1 = idx1.value;
        let idx2 = idx2.value;
        let fst =
            if idx1 < self.start.value { idx1 + self.max_size } else { idx1 };
        let snd =
            if idx2 < self.start.value { idx2 + self.max_size } else { idx2 };
        fst < snd
    }

    pub fn index_is_smaller_or_equal(&self, idx_1: WindowIndex,
                                     idx_2: WindowIndex) -> bool {
        !self.index_is_smaller(idx_2, idx_1)
    }

    pub fn for_each_suffix<F: Fn(WindowIndex) -> ()>(&self, action: F) {
        let suffices_range =
            if self.size == 0 || self.start.value <= self.cursor.value {
                (self.start.value..self.cursor.value + 1).chain(0..0)
            } else {
                (self.start.value..self.max_size)
                    .chain(0..self.cursor.value + 1)
            };
        assert_eq!(suffices_range.clone().count(), self.size);
        for suffix_start in suffices_range {
            action(WindowIndex::new(suffix_start));
        }
    }

    pub fn compare_for_equal_prefix(&self, starting_index_first: WindowIndex,
                                    starting_index_second: WindowIndex,
                                    bit_index: usize,
                                    full_byte_length: usize) -> bool {
        let starting_index_first = starting_index_first.value;
        let starting_index_second = starting_index_second.value;
        if starting_index_first.max(starting_index_second) + full_byte_length
            < self.max_size {
            self.compare_for_equal_prefix_straight(
                starting_index_first, starting_index_second,
                bit_index, full_byte_length)
        } else {
            let mut index_first = WindowIndex::new(starting_index_first);
            let mut index_second = WindowIndex::new(starting_index_second);
            let mut equal = true;
            for _ in 0..full_byte_length {
                equal &= self[index_first] == self[index_second];
                if !equal { break; }
                index_first = self.index_increment(index_first);
                index_second = self.index_increment(index_second);
            }
            equal &= self.compare_for_equal_prefix_straight(
                index_first.value, index_second.value, bit_index, 0);
            equal
        }
    }

    fn compare_for_equal_prefix_straight(&self,
                                         starting_index_first: usize,
                                         starting_index_second: usize,
                                         bit_index: usize,
                                         full_byte_length: usize) -> bool {
        let mut equal = true;
        for position in 0..full_byte_length {
            equal &= self.buffer[starting_index_first + position] ==
                self.buffer[starting_index_second + position];
            if !equal { break; };
        }
        let mut bit_position = 7;
        while equal && bit_position > bit_index {
            equal &= !self.bytes_differ_on(
                WindowIndex::new(starting_index_first + full_byte_length),
                WindowIndex::new(starting_index_second + full_byte_length),
                bit_position);
            bit_position -= 1;
        }
        equal
    }

    pub fn bytes_differ_on(&self, first_byte_index: WindowIndex,
                           second_byte_index: WindowIndex,
                           bit_index: usize) -> bool {
        get_bit(self[first_byte_index] ^ self[second_byte_index], bit_index)
    }

    pub fn get_bit(&self, byte_index: WindowIndex, bit_index: usize) -> bool {
        get_bit(self[byte_index], bit_index)
    }

    pub fn set_bit_at_cursor(&mut self, bit_value: bool, bit_index: usize) {
        self.buffer[self.cursor.raw()] &= !(1 << bit_index);
        self.buffer[self.cursor.raw()] |= (bit_value as u8) << bit_index;
    }

    pub fn set_byte_at_cursor(&mut self, byte_value: u8) {
        self.buffer[self.cursor.raw()] = byte_value;
    }
}

impl ops::Index<WindowIndex> for InputWindow {
    type Output = u8;

    fn index(&self, window_index: WindowIndex) -> &u8 {
        &self.buffer[window_index.value]
    }
}
