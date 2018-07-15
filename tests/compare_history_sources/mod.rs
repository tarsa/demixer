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
use demixer::estimators::cost::CostTracker;
use demixer::history::{
    ContextState, CollectedContextStates, HistorySource,
};
use demixer::history::naive::NaiveHistorySource;
use demixer::history::fat_map::FatMapHistorySource;
use demixer::history::tree::TreeHistorySource;
use demixer::history::tree::node::CostTrackers;
use demixer::lut::LookUpTables;
use demixer::random::MersenneTwister;

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

    let mut prng = MersenneTwister::default();

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
                assert_eq!(neutralize_cost_trackers(&naive_source_results),
                           neutralize_cost_trackers(&fat_map_source_results),
                           "index = {}, bit index = {}, input = {:?}",
                           index, bit_index, input);
            }
            if PRINT_DEBUG {
                println!("active contexts = {}", tree_source.active_contexts);
                println!("before: index = {}, bit index = {}, input = {:?}",
                         index, bit_index, input);
            }
            assert_eq!(fat_map_source_results.items(),
                       tree_source_results.items(),
                       "index = {}, bit index = {}, input = {:?}",
                       index, bit_index, input);

            let mut neutral_cost_trackers = Vec::new();
            let mut random_cost_trackers = Vec::new();
            for _ in tree_source_results.items().iter()
                .filter(|c| c.is_for_node()) {
                neutral_cost_trackers.push(CostTrackers::new(
                    CostTracker::INITIAL, CostTracker::INITIAL));
                random_cost_trackers.push(CostTrackers::new(
                    CostTracker::new(prng.next_int64() as u16),
                    CostTracker::new(prng.next_int64() as u16)));
            }

            let input_bit: Bit = ((byte & (1 << bit_index)) != 0).into();
            if PRINT_DEBUG { println!("processing bit: {}", input_bit); }
            if run_naive {
                naive_source.process_input_bit(input_bit,
                                               &neutral_cost_trackers);
            }
            fat_map_source.process_input_bit(input_bit, &random_cost_trackers);
            tree_source.process_input_bit(input_bit, &random_cost_trackers);
            if PRINT_DEBUG { println!(); }
        }
    }
}

fn neutralize_cost_trackers(context_states: &CollectedContextStates)
                            -> Vec<ContextState> {
    context_states.items().iter().map(|ctx| {
        let mut ctx = ctx.clone();
        ctx.neutralize_cost_trackers();
        ctx
    }).collect()
}
