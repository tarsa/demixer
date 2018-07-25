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
pub mod chain;
pub mod post_process;
pub mod single;
pub mod stats;

use bit::Bit;
use coding::FinalProbability;
use history::{CollectedContextStates, HistorySource};
use history::tree::TreeHistorySource;
use lut::LookUpTables;
use util::last_bytes::LastBytesCache;
use self::chain::ContextsChainPredictionMixer;
use self::post_process::PredictionFinalizer;
use self::stats::PredictionStatistics;

pub struct Predictor<'a> {
    last_bytes: LastBytesCache,
    tree_source: TreeHistorySource<'a>,
    contexts_chain: CollectedContextStates,
    contexts_chain_predictor: ContextsChainPredictionMixer<'a>,
    prediction_finalizer: PredictionFinalizer<'a>,
    final_probability_opt: Option<FinalProbability>,
    statistics: PredictionStatistics<'a>,
}

impl<'a> Predictor<'a> {
    const MAX_ORDER: usize = 20;

    pub fn new(luts: &'a LookUpTables) -> Self {
        let max_order = Self::MAX_ORDER;
        Predictor {
            last_bytes: LastBytesCache::new(),
            tree_source: TreeHistorySource::new(10_000_000, max_order, luts),
            contexts_chain: CollectedContextStates::new(max_order),
            contexts_chain_predictor: ContextsChainPredictionMixer::new(
                max_order, luts),
            prediction_finalizer: PredictionFinalizer::new(luts),
            final_probability_opt: None,
            statistics: PredictionStatistics::new(max_order, luts),
        }
    }

    pub fn start_new_byte(&mut self) {
        assert_eq!(self.final_probability_opt, None);
        self.last_bytes.start_new_byte();
        self.tree_source.start_new_byte();
        self.statistics.start_new_byte(&self.last_bytes);
    }

    pub fn predict(&mut self) -> FinalProbability {
        assert_eq!(self.final_probability_opt, None);

        self.contexts_chain.reset();
        self.tree_source.gather_history_states(&mut self.contexts_chain);

        let contexts_count = self.contexts_chain.items().len();

        let mixed_probability = self.contexts_chain_predictor
            .predict(&self.contexts_chain,
                     self.last_bytes.unfinished_byte().raw());

        let final_probability = self.prediction_finalizer.refine(
            mixed_probability.0, mixed_probability.1,
            contexts_count, &self.last_bytes);
        self.final_probability_opt = Some(final_probability);
        final_probability
    }

    pub fn update(&mut self, input_bit: Bit) {
        assert_ne!(self.final_probability_opt, None);

        let contexts_count = self.contexts_chain.items().len();

        let cost_trackers = self.contexts_chain_predictor
            .update(input_bit, &self.contexts_chain);

        self.tree_source.process_input_bit(input_bit, &cost_trackers);

        self.prediction_finalizer.update(input_bit, contexts_count,
                                         &self.last_bytes);

        self.statistics.on_next_bit(input_bit, &self.contexts_chain,
                                    self.final_probability_opt.unwrap());
        self.final_probability_opt = None;

        // updating this must be the last update step
        self.last_bytes.on_next_bit(input_bit);
    }

    pub fn print_state(&self) {
        self.statistics.print_state();
    }
}
