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
    updated_bit_history,
};
use history::tree::window::{InputWindow, WindowIndex};

#[derive(Clone)]
struct LocalContextState {
    text_start: WindowIndex,
    bit_history: u32,
}

pub struct FatMapHistorySource {
    input: InputWindow,
    bit_index: usize,
    max_order: usize,
    maps: Vec<HashMap<u64, Vec<LocalContextState>>>,
}

impl FatMapHistorySource {
    fn compute_hash(&self, order: usize) -> u64 {
        let input = &self.input;
        let map = &self.maps[(order * 8) + self.bit_index];
        let mut hasher: DefaultHasher = map.hasher().build_hasher();
        for distance in (1..order + 1).rev() {
            let index = input.index_subtract(input.cursor(), distance);
            hasher.write_u8(input[index]);
        }
        hasher.write_u16((256 + input[input.cursor()] as u16) >>
            (self.bit_index + 1));
        hasher.finish()
    }
}

impl HistorySource for FatMapHistorySource {
    fn new(max_window_size: usize, max_order: usize) -> FatMapHistorySource {
        FatMapHistorySource {
            input: InputWindow::new(max_window_size, 0),
            bit_index: 7,
            max_order,
            maps: vec![HashMap::new(); (max_order + 1) * 8],
        }
    }

    fn start_new_byte(&mut self) {
        assert_eq!(self.bit_index, 7);
        assert_ne!(self.input.size(), self.input.max_size(),
                   "input window is filled up, but sliding is not implemented");
        self.input.advance_cursor();
    }

    fn gather_history_states(&self,
                             bit_histories: &mut CollectedContextStates) {
        for order in 0..(self.max_order.min(self.input.cursor().raw()) + 1) {
            let map = &self.maps[(order * 8) + self.bit_index];
            let hash = self.compute_hash(order);
            let vec_opt = map.get(&hash);
            match vec_opt.into_iter().
                flat_map(|vec| vec.into_iter().find(|item| {
                    self.input.compare_for_equal_prefix(
                        self.input.index_subtract(self.input.cursor(), order),
                        item.text_start, self.bit_index, order)
                })).last() {
                Some(ctx) =>
                    bit_histories.items.push(ContextState {
                        last_occurrence_index: ctx.text_start,
                        bit_history: ctx.bit_history,
                    }),
                None => { break; }
            }
        }
    }

    fn process_input_bit(&mut self, input_bit: bool) {
        for order in 0..(self.max_order.min(self.input.cursor().raw()) + 1) {
            let hash = self.compute_hash(order);
            let map = &mut self.maps[(order * 8) + self.bit_index];
            let vec = map.entry(hash).or_insert(Vec::new());
            let input = &self.input;
            let byte_index = input.index_subtract(input.cursor(), order);
            let bit_index = self.bit_index;
            let found = vec.iter_mut().find(|item|
                input.compare_for_equal_prefix(
                    byte_index, item.text_start, bit_index, order)
            ).map(|ctx| {
                ctx.text_start = byte_index;
                ctx.bit_history =
                    updated_bit_history(ctx.bit_history, input_bit);
            }).is_some();
            if !found {
                vec.push(LocalContextState {
                    text_start: byte_index,
                    bit_history: 2 + input_bit as u32,
                });
            }
        }
        self.input.set_bit_at_cursor(input_bit, self.bit_index);
        if self.bit_index > 0 {
            self.bit_index -= 1;
        } else {
            self.bit_index = 7;
        }
    }
}
