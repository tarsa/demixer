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
use estimators::decelerating::DeceleratingEstimator;
use fixed_point::{FixedPoint, FixU32};
use fixed_point::types::{FractOnlyU32, Log2D, StretchedProbD};
use history::ContextState;
use history::tree::node::CostTrackers;
use lut::LookUpTables;
use mixing::mixer::{
    MixerInitializationMode, Mixer, FixedSizeMixer, Mixer1, Mixer2,
};
use util::indexer::{Indexer, Indexer2, Indexer3, Indexer4};
use util::quantizers::OccurrenceCountQuantizer;

pub struct SingleContextPredictor {
    edge_fixed_st: StretchedProbD,
    edge_fixed_sq: FractOnlyU32,
    edge_mixers: Vec<Mixer1>,
    edge_mixer_indexer: Indexer4,
    edge_mixer_index_opt: Option<usize>,
    edge_mixing_result_opt: Option<(FractOnlyU32, StretchedProbD)>,
    node_non_stationary_estimators: Vec<DeceleratingEstimator>,
    node_non_stationary_indexer: Indexer2,
    node_non_stationary_index_opt: Option<usize>,
    node_mixers: Vec<Mixer2>,
    node_mixer_indexer: Indexer3,
    node_mixer_index_opt: Option<usize>,
    node_mixing_result_opt: Option<(FractOnlyU32, StretchedProbD)>,
}

impl SingleContextPredictor {
    pub fn new(luts: &LookUpTables) -> Self {
        let edge_fixed_st = StretchedProbD::new(6 << 21, 21);
        let edge_fixed_sq = luts.squash_lut().squash(edge_fixed_st);
        let mut edge_mixer_indexer = Indexer4::new(vec![
            OccurrenceCountQuantizer::max_output() + 1, 256, 4, 2]);
        let mut node_non_stationary_indexer = Indexer2::new(vec![
            OccurrenceCountQuantizer::max_output() + 1, 2]);
        let mut node_mixer_indexer = Indexer3::new(vec![
            6, 4, OccurrenceCountQuantizer::max_output() + 1]);
        SingleContextPredictor {
            edge_fixed_st,
            edge_fixed_sq,
            edge_mixers: vec![
                Mixer1::new(0, MixerInitializationMode::AllZero);
                edge_mixer_indexer.get_array_size()],
            edge_mixer_indexer,
            edge_mixer_index_opt: None,
            edge_mixing_result_opt: None,
            node_non_stationary_estimators: vec![
                DeceleratingEstimator::new();
                node_non_stationary_indexer.get_array_size()],
            node_non_stationary_indexer,
            node_non_stationary_index_opt: None,
            node_mixers: vec![
                Mixer2::new(3, MixerInitializationMode::EqualSummingToOne);
                node_mixer_indexer.get_array_size()],
            node_mixer_indexer,
            node_mixer_index_opt: None,
            node_mixing_result_opt: None,
        }
    }

    pub fn predict(&mut self, ctx_state: &ContextState, context_byte: u8,
                   luts: &LookUpTables) -> FractOnlyU32 {
        assert_eq!(self.edge_mixer_index_opt, None);
        assert_eq!(self.edge_mixing_result_opt, None);
        assert_eq!(self.node_non_stationary_index_opt, None);
        assert_eq!(self.node_mixer_index_opt, None);
        assert_eq!(self.node_mixing_result_opt, None);
        match ctx_state {
            &ContextState::ForEdge {
                repeated_bit, occurrence_count, last_occurrence_distance
            } => {
                let mixer_index = self.edge_mixer_indexer
                    .with_sub_index(
                        OccurrenceCountQuantizer::quantize(occurrence_count))
                    .with_sub_index(context_byte as usize)
                    .with_sub_index(quantize_distance(last_occurrence_distance))
                    .with_sub_index(repeated_bit.to_u8() as usize)
                    .get_array_index_and_reset();
                self.edge_mixer_index_opt = Some(mixer_index);
                let mixer = &mut self.edge_mixers[mixer_index];
                mixer.set_input(0, self.edge_fixed_sq, self.edge_fixed_st);
                let mixing_result = mixer.mix_all(luts.squash_lut());
                self.edge_mixing_result_opt = Some(mixing_result);
                mixing_result.0
            }
            &ContextState::ForNode {
                last_occurrence_distance, probability_estimator, bits_runs,
                ref cost_trackers, ..
            } => {
                let non_stationary_index = self.node_non_stationary_indexer
                    .with_sub_index(OccurrenceCountQuantizer::quantize(
                        bits_runs.last_bit_run_length()))
                    .with_sub_index(bits_runs.last_bit().to_u8() as usize)
                    .get_array_index_and_reset();
                self.node_non_stationary_index_opt =
                    Some(non_stationary_index);
                let non_stationary_prediction_sq =
                    self.node_non_stationary_estimators[non_stationary_index]
                        .prediction();
                let non_stationary_prediction_st =
                    luts.stretch_lut().stretch(non_stationary_prediction_sq);
                let stationary_prediction_sq =
                    probability_estimator.prediction();
                let stationary_prediction_st =
                    luts.stretch_lut().stretch(stationary_prediction_sq);
                let quantized_usage_count = OccurrenceCountQuantizer::quantize(
                    probability_estimator.usage_count());
                let mixer_index = self.node_mixer_indexer
                    .with_sub_index(quantize_cost_trackers(cost_trackers))
                    .with_sub_index(quantize_distance(last_occurrence_distance))
                    .with_sub_index(quantized_usage_count)
                    .get_array_index_and_reset();
                self.node_mixer_index_opt = Some(mixer_index);
                let mixer = &mut self.node_mixers[mixer_index];
                mixer.set_input(0, stationary_prediction_sq,
                                stationary_prediction_st);
                mixer.set_input(1, non_stationary_prediction_sq,
                                non_stationary_prediction_st);
                let mixing_result = mixer.mix_all(luts.squash_lut());
                self.node_mixing_result_opt = Some(mixing_result);
                mixing_result.0
            }
        }
    }

    pub fn update(&mut self, ctx_state: &ContextState, input_bit: Bit,
                  luts: &LookUpTables) -> Option<CostTrackers> {
        match ctx_state {
            &ContextState::ForEdge { .. } => {
                let mixer_index = self.edge_mixer_index_opt.unwrap();
                self.edge_mixer_index_opt = None;
                let mixing_result = self.edge_mixing_result_opt.unwrap();
                self.edge_mixing_result_opt = None;
                self.edge_mixers[mixer_index].update_and_reset(
                    input_bit, mixing_result.0, 500, luts.d_estimator_rates());
                None
            }
            &ContextState::ForNode { ref cost_trackers, .. } => {
                let non_stationary_index =
                    self.node_non_stationary_index_opt.unwrap();
                self.node_non_stationary_index_opt = None;
                self.node_non_stationary_estimators[non_stationary_index]
                    .update(input_bit, luts.d_estimator_rates());
                let mixing_result = self.node_mixing_result_opt.unwrap();
                self.node_mixing_result_opt = None;
                let mixer_index = self.node_mixer_index_opt.unwrap();
                self.node_mixer_index_opt = None;
                let mixer = &mut self.node_mixers[mixer_index];
                let new_cost_trackers = CostTrackers::new(
                    cost_trackers.stationary().updated(estimate_cost(
                        mixer.prediction_sq(0), input_bit, luts)),
                    cost_trackers.non_stationary().updated(estimate_cost(
                        mixer.prediction_sq(1), input_bit, luts)),
                );
                mixer.update_and_reset(
                    input_bit, mixing_result.0, 100, luts.d_estimator_rates());
                Some(new_cost_trackers)
            }
        }
    }
}

fn quantize_distance(distance: usize) -> usize {
    if distance < 100 {
        0
    } else if distance < 1_000 {
        1
    } else if distance < 10_000 {
        2
    } else {
        3
    }
}

fn quantize_cost_trackers(cost_trackers: &CostTrackers) -> usize {
    let s = cost_trackers.stationary().raw() as u32;
    let n = cost_trackers.non_stationary().raw() as u32;
    if s + s / 2 < n {
        0
    } else if s + s / 8 < n {
        1
    } else if s < n {
        2
    } else if s < n + n / 8 {
        3
    } else if s < n + n / 2 {
        4
    } else {
        5
    }
}

fn estimate_cost(probability_sq: FractOnlyU32, input_bit: Bit,
                 luts: &LookUpTables) -> Log2D {
    probability_sq.to_fix_u32::<FinalProbability>()
        .estimate_cost(input_bit, luts.log2_lut())
}
