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
use bit::Bit;
use coding::FinalProbability;
use fixed_point::{FixedPoint, FixI32, FixI64};
use fixed_point::types::Log2Q;
use history::CollectedContextStates;
use lut::LookUpTables;

pub struct PredictionStatistics<'a> {
    luts: &'a LookUpTables,
    total_bytes: u64,
    total_contexts_count: u64,
    per_order_counts: Vec<usize>,
    per_order_costs: Vec<Log2Q>,
    total_cost: Log2Q,
    per_order_occurrences_count: Vec<u64>,
}

impl<'a> PredictionStatistics<'a> {
    pub fn new(max_order: usize, luts: &'a LookUpTables) -> Self {
        PredictionStatistics {
            luts,
            total_bytes: 0,
            total_contexts_count: 0,
            per_order_counts: vec![0; 5 * 5],
            per_order_costs: vec![Log2Q::new_unchecked(0); 5 * 5],
            total_cost: Log2Q::new_unchecked(0),
            per_order_occurrences_count: vec![0; 5 * (max_order + 1)],
        }
    }

    pub fn start_new_byte(&mut self) {
        self.total_bytes += 1;
    }

    pub fn on_next_bit(&mut self, input_bit: Bit,
                       contexts: &CollectedContextStates,
                       final_probability: FinalProbability) {
        let contexts_count = contexts.items().len();
        self.total_contexts_count += contexts_count as u64;

        if contexts_count > 0 {
            let max_order = contexts_count - 1;
            let stats_index = {
                let orders = 4.min(max_order);
                let unary_orders = 4.min(contexts.items().iter()
                    .filter(|ctx| !ctx.is_for_node()).count());
                orders * 5 + unary_orders
            };
            self.per_order_counts[stats_index] += 1;
            let cost = final_probability
                .estimate_cost(input_bit, &self.luts.log2_lut()).to_fix_i64();
            self.total_cost = self.total_cost.add(&cost);
            self.per_order_costs[stats_index] =
                self.per_order_costs[stats_index].add(&cost);
            self.per_order_occurrences_count[stats_index] +=
                contexts.items()[max_order].occurrence_count() as u64;
        }
    }

    pub fn print_state(&self) {
        println!("Predictor state:");
        let average_context_length =
            self.total_contexts_count as f64 / (self.total_bytes * 8) as f64;
        println!("average context length = {:15.3}", average_context_length);
        println!("Costs histogram:");
        for order in 0..=4 {
            for unary_order in 0..=4 {
                let mixer_index = order * 5 + unary_order;
                let average_occurrence_count =
                    self.per_order_occurrences_count[mixer_index] as f64 /
                        self.per_order_counts[mixer_index] as f64;
                println!("order {}, unary {}, count {:10.2}, \
                          cost {:10.2}, avg occur {:8.3}",
                         order, unary_order,
                         self.per_order_counts[mixer_index] as f64 / 8.0,
                         self.per_order_costs[mixer_index].as_f64() / 8.0,
                         average_occurrence_count);
            }
        }
        println!("Total cost (computed using LUTs): {:10.2} bytes",
                 self.total_cost.as_f64() / 8.0);
    }
}
