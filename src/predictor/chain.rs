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
use mixing::mixer::{Mixer, FixedSizeMixer, Mixer2};
use util::indexer::{Indexer, Indexer5};
use super::single::SingleContextPredictor;

pub struct ContextsChainPredictionMixer<'a> {
    luts: &'a LookUpTables,
    single_context_predictors: Vec<SingleContextPredictor>,
    mixers: Vec<Mixer2>,
    mixer_indexer: Indexer5,
    mixer_indices: Vec<i32>,
    mixed_probability_opt: Option<(FractOnlyU32, StretchedProbD)>,
}

impl<'a> ContextsChainPredictionMixer<'a> {
    const INTERVALS_SCALE: i32 = 8;

    const MAX_COMPARE_RESULT: usize = 4;

    pub fn new(max_order: usize, luts: &'a LookUpTables) -> Self {
        let mut single_context_predictors = Vec::with_capacity(max_order + 1);
        for _ in 0..=max_order {
            single_context_predictors.push(SingleContextPredictor::new(luts));
        }
        let intervals_number = (StretchedProbD::intervals_count(0) /
            Self::INTERVALS_SCALE) as usize;
        let mut mixer_indexer = Indexer5::new(vec![
            2,
            max_order + 1,
            intervals_number,
            intervals_number,
            Self::MAX_COMPARE_RESULT + 1,
        ]);
        ContextsChainPredictionMixer {
            luts,
            single_context_predictors,
            mixers: vec![Mixer2::new(8, false); mixer_indexer.get_array_size()],
            mixer_indexer,
            mixer_indices: vec![-1; max_order + 1],
            mixed_probability_opt: None,
        }
    }

    pub fn predict(&mut self, contexts: &CollectedContextStates,
                   context_byte: u8) -> (FractOnlyU32, StretchedProbD) {
        assert_eq!(self.mixed_probability_opt, None);
        let mut last_probability = (FractOnlyU32::HALF, StretchedProbD::ZERO);
        for (order, context) in contexts.items().iter().enumerate() {
            let max_order = contexts.items().len() - 1;
            let (previous_probability_sq, previous_probability_st) =
                last_probability;
            let current_probability_sq = self.single_context_predictors[order]
                .predict(context, context_byte, self.luts);
            let current_probability_st =
                self.luts.stretch_lut().stretch(current_probability_sq);
            let context_states_compare_result = if order == 0 { 0 } else {
                self.compare_context_states(&contexts.items()[order - 1],
                                            &contexts.items()[order])
            };
            let mixer_index = self.mixer_indexer
                .with_sub_index((max_order == order) as usize)
                .with_sub_index(order)
                .with_sub_index((current_probability_st.to_interval_index(0) /
                    Self::INTERVALS_SCALE) as usize)
                .with_sub_index((previous_probability_st.to_interval_index(0) /
                    Self::INTERVALS_SCALE) as usize)
                .with_sub_index(context_states_compare_result)
                .get_array_index_and_reset();
            assert_eq!(self.mixer_indices[order], -1);
            self.mixer_indices[order] = mixer_index as i32;
            let mixer = &mut self.mixers[mixer_index];
            mixer.set_input(
                0, previous_probability_sq, previous_probability_st);
            mixer.set_input(1, current_probability_sq, current_probability_st);
            last_probability = mixer.mix_all(self.luts.squash_lut());
        }
        self.mixed_probability_opt = Some(last_probability);
        last_probability
    }

    pub fn update(&mut self, input_bit: Bit, contexts: &CollectedContextStates)
                  -> Vec<CostTrackers> {
        assert_ne!(self.mixed_probability_opt, None);
        let mixed_probability = self.mixed_probability_opt.unwrap();
        self.mixed_probability_opt = None;

        let mut cost_trackers = Vec::new();
        for (order, context) in contexts.items().iter().enumerate() {
            let max_order = contexts.items().len() - 1;
            let mixed_sq =
                if order == max_order {
                    mixed_probability.0
                } else {
                    let upper_mixer_index = self.mixer_indices[order + 1];
                    assert_ne!(upper_mixer_index, -1);
                    self.mixers[upper_mixer_index as usize].prediction_sq(0)
                };
            let mixer_index = self.mixer_indices[order];
            self.mixer_indices[order] = -1;
            assert_ne!(mixer_index, -1);
            let mixer = &mut self.mixers[mixer_index as usize];
            mixer.update_and_reset(
                input_bit, mixed_sq, 300, self.luts.d_estimator_rates());

            let cost_trackers_opt = self.single_context_predictors[order]
                .update(context, input_bit, self.luts);
            assert_ne!(cost_trackers_opt == None, context.is_for_node());
            cost_trackers.extend(cost_trackers_opt.into_iter());
        }
        cost_trackers
    }

    fn compare_context_states(&self, previous: &ContextState,
                              current: &ContextState) -> usize {
        match (previous.is_for_node(), current.is_for_node()) {
            (false, false) =>
                0 + (previous.occurrence_count() ==
                    current.occurrence_count()) as usize,
            (true, true) =>
                2 + (previous.bit_history(self.luts) ==
                    current.bit_history(self.luts)) as usize,
            (true, false) => 4,
            _ => panic!("binary context cannot be longer than unary context"),
        }
    }
}
