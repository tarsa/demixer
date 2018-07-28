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
use fixed_point::types::{FractOnlyU32, StretchedProbD};
use history::{ContextState, CollectedContextStates};
use history::tree::node::CostTrackers;
use lut::LookUpTables;
use mixing::mixer::{Mixer3, Mixer4};
use util::indexer::{Indexer, Indexer4, Indexer5};
use super::kits::MixersWithIndexer;
use super::single::SingleContextPredictor;

pub struct ContextsChainPredictionMixer<'a> {
    luts: &'a LookUpTables,
    single_context_predictors: Vec<SingleContextPredictor>,
    edge_mixer_kits: Vec<MixersWithIndexer<Mixer4, Indexer4>>,
    node_mixer_kits: Vec<MixersWithIndexer<Mixer3, Indexer5>>,
}

impl<'a> ContextsChainPredictionMixer<'a> {
    const MAX_COMPARE_RESULT: usize = 2;

    pub fn new(max_order: usize, luts: &'a LookUpTables) -> Self {
        let make_dimensions = |extra_dimensions: Vec<usize>| {
            let mut dimensions = vec![2, Self::MAX_COMPARE_RESULT + 1];
            dimensions.extend(extra_dimensions.into_iter());
            dimensions
        };
        let mut single_context_predictors = Vec::with_capacity(max_order + 1);
        let mut edge_mixer_kits = Vec::with_capacity(max_order + 1);
        let mut node_mixer_kits = Vec::with_capacity(max_order + 1);
        for _ in 0..=max_order {
            single_context_predictors.push(SingleContextPredictor::new(luts));
            let edge_mixer_kit = {
                let extra_dimensions =
                    SingleContextPredictor::edge_mixer_extra_dimensions();
                MixersWithIndexer::new(
                    || SingleContextPredictor::new_edge_mixer(),
                    Indexer4::new(make_dimensions(extra_dimensions)))
            };
            edge_mixer_kits.push(edge_mixer_kit);
            let node_mixer_kit = {
                let extra_dimensions =
                    SingleContextPredictor::node_mixer_extra_dimensions();
                MixersWithIndexer::new(
                    || SingleContextPredictor::new_node_mixer(),
                    Indexer5::new(make_dimensions(extra_dimensions)))
            };
            node_mixer_kits.push(node_mixer_kit);
        }
        ContextsChainPredictionMixer {
            luts,
            single_context_predictors,
            edge_mixer_kits,
            node_mixer_kits,
        }
    }

    pub fn predict(&mut self, contexts: &CollectedContextStates,
                   context_byte: u8) -> (FractOnlyU32, StretchedProbD) {
        let mut last_probability = (FractOnlyU32::HALF, StretchedProbD::ZERO);
        for (order, context) in contexts.items().iter().enumerate() {
            let max_order = contexts.items().len() - 1;
            let is_max_order_result = (max_order == order) as usize;
            let context_states_compare_result = if order == 0 { 0 } else {
                self.compare_context_states(&contexts.items()[order - 1],
                                            &contexts.items()[order])
            };
            let current_probability =
                if context.is_for_node() {
                    let mixer_kit = &mut self.node_mixer_kits[order];
                    mixer_kit.pre_predict(|indexer| indexer
                        .with_sub_index(is_max_order_result)
                        .with_sub_index(context_states_compare_result));
                    self.single_context_predictors[order].predict(
                        mixer_kit, context, context_byte, last_probability,
                        self.luts)
                } else {
                    let mixer_kit = &mut self.edge_mixer_kits[order];
                    mixer_kit.pre_predict(|indexer| indexer
                        .with_sub_index(is_max_order_result)
                        .with_sub_index(context_states_compare_result));
                    self.single_context_predictors[order].predict(
                        mixer_kit, context, context_byte, last_probability,
                        self.luts)
                };
            last_probability = current_probability;
        }
        last_probability
    }

    pub fn update(&mut self, input_bit: Bit, contexts: &CollectedContextStates)
                  -> Vec<CostTrackers> {
        let mut cost_trackers = Vec::new();
        for (order, context) in contexts.items().iter().enumerate() {
            let cost_trackers_opt =
                if context.is_for_node() {
                    let mixer_kit = &mut self.node_mixer_kits[order];
                    self.single_context_predictors[order]
                        .update(mixer_kit, context, input_bit, self.luts)
                } else {
                    let mixer_kit = &mut self.edge_mixer_kits[order];
                    self.single_context_predictors[order]
                        .update(mixer_kit, context, input_bit, self.luts)
                };
            assert_ne!(cost_trackers_opt == None, context.is_for_node());
            cost_trackers.extend(cost_trackers_opt.into_iter());
        }
        cost_trackers
    }

    fn compare_context_states(&self, previous: &ContextState,
                              current: &ContextState) -> usize {
        match (previous.is_for_node(), current.is_for_node()) {
            (false, false) | (true, true) =>
                (previous.probability_estimator(self.luts) ==
                    current.probability_estimator(self.luts)) as usize,
            (true, false) => 2,
            _ => panic!("binary context cannot be longer than unary context"),
        }
    }
}
