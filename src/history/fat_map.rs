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

use bit::Bit;
use lut::LookUpTables;
use super::{
    ContextState, CollectedContextStates, HistorySource,
};
use super::window::{WindowIndex, InputWindow};
use super::tree::node::CostTrackers;

/// NOTE: last_occurrence_distance in fact holds last occurrence index
pub struct FatMapHistorySource<'a> {
    luts: &'a LookUpTables,
    input: InputWindow,
    bit_index: usize,
    max_order: usize,
    maps: Vec<HashMap<u64, Vec<ContextState>>>,
}

impl<'a> FatMapHistorySource<'a> {
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

impl<'a> HistorySource<'a> for FatMapHistorySource<'a> {
    fn new(max_window_size: usize, max_order: usize, luts: &'a LookUpTables)
           -> FatMapHistorySource {
        FatMapHistorySource {
            luts,
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
                        WindowIndex::new(item.last_occurrence_distance()),
                        self.bit_index, order)
                })).last() {
                Some(ctx) => {
                    let mut ctx = ctx.clone();
                    let actual_distance = self.input.cursor().raw() - order
                        - ctx.last_occurrence_distance();
                    match &mut ctx {
                        &mut ContextState::ForEdge {
                            ref mut last_occurrence_distance, ..
                        } => *last_occurrence_distance = actual_distance,
                        &mut ContextState::ForNode {
                            ref mut last_occurrence_distance, ..
                        } => *last_occurrence_distance = actual_distance,
                    };
                    bit_histories.items.push(ctx)
                }
                None => { break; }
            }
        }
    }

    fn process_input_bit(&mut self, input_bit: Bit,
                         new_cost_trackers: &[CostTrackers]) {
        for order in 0..(self.max_order.min(self.input.cursor().raw()) + 1) {
            let hash = self.compute_hash(order);
            let map = &mut self.maps[(order * 8) + self.bit_index];
            let vec = map.entry(hash).or_insert(Vec::new());
            let input = &self.input;
            let byte_index = input.index_subtract(input.cursor(), order);
            let bit_index = self.bit_index;
            let luts = self.luts;
            let found = vec.iter_mut()
                .find(|item| {
                    let last_occurrence_index =
                        WindowIndex::new(item.last_occurrence_distance());
                    input.compare_for_equal_prefix(
                        byte_index, last_occurrence_index, bit_index, order)
                })
                .map(|ctx| {
                    let cost_trackers_opt =
                        if ctx.is_for_node() {
                            assert!(order < new_cost_trackers.len());
                            Some(new_cost_trackers[order].clone())
                        } else {
                            assert!(order >= new_cost_trackers.len());
                            None
                        };
                    *ctx = ctx.next_state(byte_index.raw(), input_bit,
                                          cost_trackers_opt, luts)
                })
                .is_some();
            if !found {
                assert!(order >= new_cost_trackers.len());
                vec.push(ContextState::starting_state(byte_index.raw(),
                                                      input_bit));
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
