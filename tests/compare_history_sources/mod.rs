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
extern crate demixer;

use demixer::PRINT_DEBUG;
use demixer::bit::Bit;
use demixer::history::{
    HistorySource,
    CollectedContextStates,
    ContextState,
};
use demixer::history::naive::NaiveHistorySource;
use demixer::history::fat_map::FatMapHistorySource;
use demixer::history::tree::TreeHistorySource;
use demixer::lut::LookUpTables;

pub fn compare_for_input(input: &[u8], max_order: usize, run_naive: bool,
                         luts: &LookUpTables) {
    let mut naive_source =
        NaiveHistorySource::new(input.len(), max_order, luts);
    let mut fat_map_source =
        FatMapHistorySource::new(input.len(), max_order, luts);
    let mut tree_source = TreeHistorySource::new_special(
        input.len(), max_order, input.len() / 2, luts);

    let mut naive_source_results = CollectedContextStates::new(max_order);
    let mut fat_map_source_results = CollectedContextStates::new(max_order);
    let mut tree_source_results = CollectedContextStates::new(max_order);

    for (index, byte) in input.iter().enumerate() {
        if run_naive {
            naive_source.start_new_byte();
        }
        fat_map_source.start_new_byte();
        tree_source.start_new_byte();
        if PRINT_DEBUG {
            println!("started byte #{}, max order = {}", index, max_order);
            tree_source.tree.print();
        }
        tree_source.check_integrity_on_next_byte();

        for bit_index in (0..7 + 1).rev() {
            if run_naive {
                naive_source_results.reset();
                naive_source.gather_history_states(&mut naive_source_results);
            }

            fat_map_source_results.reset();
            fat_map_source.gather_history_states(&mut fat_map_source_results);

            tree_source_results.reset();
            tree_source.gather_history_states(&mut tree_source_results);

            if run_naive {
                assert_eq!(naive_source_results.items(),
                           fat_map_source_results.items(),
                           "index = {}, bit index = {}, input = {:?}",
                           index, bit_index, input);
            }
            if PRINT_DEBUG {
                println!("active contexts = {}", tree_source.active_contexts);
                println!("before: index = {}, bit index = {}, input = {:?}",
                         index, bit_index, input);
            }
            assert_eq!(fat_map_source_results.items(),
                       &tree_source_results.items().iter().map(|ctx_state| {
                           let last_occurrence_index = tree_source.tree.window
                               .index_subtract(
                                   ctx_state.last_occurrence_index(),
                                   tree_source.tree.window.start().raw());
                           match ctx_state {
                               &ContextState::ForEdge {
                                   occurrence_count, repeated_bit, ..
                               } => ContextState::ForEdge {
                                   last_occurrence_index,
                                   occurrence_count,
                                   repeated_bit,
                               },
                               &ContextState::ForNode {
                                   probability_estimator, bit_history, ..
                               } => ContextState::ForNode {
                                   last_occurrence_index,
                                   probability_estimator,
                                   bit_history,
                               },
                           }
                       }).collect::<Vec<_>>()[..],
                       "index = {}, bit index = {}, input = {:?}",
                       index, bit_index, input);

            let input_bit: Bit = ((byte & (1 << bit_index)) != 0).into();
            if PRINT_DEBUG { println!("processing bit: {}", input_bit); }
            if run_naive {
                naive_source.process_input_bit(input_bit);
            }
            fat_map_source.process_input_bit(input_bit);
            tree_source.process_input_bit(input_bit);
            if PRINT_DEBUG { println!(); }
        }
    }
}
