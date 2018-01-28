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
use core::hash::BuildHasher;
use core::hash::Hasher;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;

use history::{
    HistorySource,
    ContextState,
    CollectedContextStates,
    updated_bit_history, compare_for_equal_prefix,
};

#[derive(Clone)]
struct LocalContextState {
    byte_index: usize,
    bit_history: u32,
}

pub struct FatMapHistorySource {
    input: Vec<u8>,
    input_cursor: usize,
    bit_index: usize,
    max_order: usize,
    maps: Vec<HashMap<u64, Vec<LocalContextState>>>,
}

impl FatMapHistorySource {
    fn compute_hash(&self, order: usize) -> u64 {
        let map = &self.maps[(order * 8) + self.bit_index];
        let mut hasher: DefaultHasher = map.hasher().build_hasher();
        hasher.write(
            &self.input[self.input_cursor - order..self.input_cursor]);
        hasher.write_u32((256 + self.input[self.input_cursor] as u32) >>
            (self.bit_index + 1));
        hasher.finish()
    }
}

impl HistorySource for FatMapHistorySource {
    fn new(input_size: usize, max_order: usize) -> FatMapHistorySource {
        FatMapHistorySource {
            input: Vec::with_capacity(input_size),
            input_cursor: 0,
            bit_index: 7,
            max_order,
            maps: vec![HashMap::new(); (max_order + 1) * 8],
        }
    }

    fn start_new_byte(&mut self) {
        assert_eq!(self.bit_index, 7);
        assert_eq!(self.input_cursor, self.input.len());
        assert_ne!(self.input.len(), self.input.capacity());
        self.input.push(0);
    }

    fn gather_history_states(&self,
                             bit_histories: &mut CollectedContextStates) {
        for order in 0..(self.max_order.min(self.input_cursor) + 1) {
            let map = &self.maps[(order * 8) + self.bit_index];
            let hash = self.compute_hash(order);
            let vec_opt: Option<&Vec<_>> = map.get(&hash);
            match vec_opt.into_iter().
                flat_map(|vec| vec.into_iter().find(|item| {
                    compare_for_equal_prefix(
                        &self.input, self.input_cursor - order,
                        item.byte_index, self.bit_index, order)
                })).last() {
                Some(ctx) =>
                    bit_histories.items.push(ContextState {
                        first_occurrence_index: ctx.byte_index,
                        bit_history: ctx.bit_history,
                    }),
                None => break,
            };
        }
    }

    fn process_input_bit(&mut self, input_bit: bool) {
        for order in 0..(self.max_order.min(self.input_cursor) + 1) {
            let hash = self.compute_hash(order);
            let map = &mut self.maps[(order * 8) + self.bit_index];
            let vec: &mut Vec<_> = map.entry(hash).or_insert(Vec::new());
            let input = &self.input;
            let byte_index = self.input_cursor - order;
            let bit_index = self.bit_index;
            let found = vec.iter_mut().find(|item| compare_for_equal_prefix(
                input, byte_index, item.byte_index, bit_index, order)
            ).map(|ctx| ctx.bit_history =
                updated_bit_history(ctx.bit_history, input_bit)
            ).is_some();
            if !found {
                vec.push(LocalContextState {
                    byte_index,
                    bit_history: 2 + input_bit as u32,
                });
            };
        }
        self.input[self.input_cursor] |= (input_bit as u8) << self.bit_index;
        if self.bit_index > 0 {
            self.bit_index -= 1;
        } else {
            self.bit_index = 7;
            self.input_cursor += 1;
        }
    }
}
