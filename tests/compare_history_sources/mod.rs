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
use demixer::history::{
    HistorySource,
    CollectedContextStates,
};
use demixer::history::naive::NaiveHistorySource;
use demixer::history::fat_map::FatMapHistorySource;
use demixer::history::tree::TreeHistorySource;

pub fn compare_for_input(input: &[u8], max_order: usize, run_naive: bool) {
    let mut naive_source = NaiveHistorySource::new(input.len(), max_order);
    let mut fat_map_source = FatMapHistorySource::new(input.len(), max_order);
    let mut tree_source = TreeHistorySource::new(input.len(), max_order);

    let mut naive_source_results = CollectedContextStates::new(max_order);
    let mut fat_map_source_results = CollectedContextStates::new(max_order);
    let mut tree_source_results = CollectedContextStates::new(max_order);

    for (index, byte) in input.iter().enumerate() {
        if run_naive {
            naive_source.start_new_byte();
        }
        fat_map_source.start_new_byte();
        tree_source.start_new_byte();

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
                           "max order = {}, index = {}, bit index = {}, \
                           input = {:?}",
                           max_order, index, bit_index, input);
            }
            if PRINT_DEBUG {
                println!("active contexts = {:?}", tree_source.active_contexts);
                println!("max order = {}, index = {}, bit index = {}, \
                       input = {:?}",
                         max_order, index, bit_index, input);
                tree_source.tree.print();
            }
            assert_eq!(fat_map_source_results.items(),
                       tree_source_results.items(),
                       "max order = {}, index = {}, bit index = {}, \
                       input = {:?}",
                       max_order, index, bit_index, input);

            let input_bit = (byte & (1 << bit_index)) != 0;
            if run_naive {
                naive_source.process_input_bit(input_bit);
            }
            fat_map_source.process_input_bit(input_bit);
            tree_source.process_input_bit(input_bit);
        }
    }
}
