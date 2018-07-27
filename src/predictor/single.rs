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
use fixed_point::FixedPoint;
use fixed_point::types::{FractOnlyU32, StretchedProbD};
use history::ContextState;
use history::state::bits_runs::BitsRunsTracker;
use history::tree::node::CostTrackers;
use lut::LookUpTables;
use mixing::mixer::{
    MixerInitializationMode, Mixer, FixedSizeMixer, Mixer2, Mixer3,
};
use util::indexer::{Indexer, Indexer2, Indexer3, Indexer4};
use util::quantizers::OccurrenceCountQuantizer;
use super::kits::{EstimatorsWithIndexer, MixersWithIndexer};

pub struct SingleContextPredictor {
    edge_fixed_prediction: (FractOnlyU32, StretchedProbD),
    edge_mixer_kit: MixersWithIndexer<Mixer2, Indexer4>,
    node_non_stationary_1: EstimatorsWithIndexer<Indexer2>,
    node_non_stationary_3: EstimatorsWithIndexer<Indexer4>,
    node_mixer_kit: MixersWithIndexer<Mixer3, Indexer3>,
}

impl SingleContextPredictor {
    pub fn new(luts: &LookUpTables) -> Self {
        let max_quantized_bit_run_length = OccurrenceCountQuantizer::quantize(
            BitsRunsTracker::MAX_RUN_LENGTH);
        let edge_fixed_st = StretchedProbD::new(6 << 21, 21);
        let edge_fixed_sq = luts.squash_lut().squash(edge_fixed_st);
        SingleContextPredictor {
            edge_fixed_prediction: (edge_fixed_sq, edge_fixed_st),
            edge_mixer_kit: MixersWithIndexer::new(
                || Mixer2::new(0, MixerInitializationMode::DominantFirst),
                Indexer4::new(vec![
                    OccurrenceCountQuantizer::max_output() + 1, 256, 4, 2])),
            node_non_stationary_1: EstimatorsWithIndexer::new(
                Indexer2::new(vec![
                    max_quantized_bit_run_length + 1, 2])),
            node_non_stationary_3: EstimatorsWithIndexer::new(
                Indexer4::new(vec![
                    max_quantized_bit_run_length + 1,
                    max_quantized_bit_run_length + 1,
                    max_quantized_bit_run_length + 1,
                    2])),
            node_mixer_kit: MixersWithIndexer::new(
                || Mixer3::new(3, MixerInitializationMode::EqualSummingToOne),
                Indexer3::new(vec![
                    6, 4, OccurrenceCountQuantizer::max_output() + 1])),
        }
    }

    pub fn predict(&mut self, ctx_state: &ContextState, context_byte: u8,
                   luts: &LookUpTables) -> FractOnlyU32 {
        match ctx_state {
            &ContextState::ForEdge {
                repeated_bit, occurrence_count, last_occurrence_distance
            } => {
                let direct = luts.direct_predictions()
                    .for_0_bit_run(occurrence_count);
                let edge_fixed = self.edge_fixed_prediction;
                let mixing_result = self.edge_mixer_kit.predict(
                    |indexer| indexer
                        .with_sub_index(OccurrenceCountQuantizer::quantize(
                            occurrence_count))
                        .with_sub_index(context_byte as usize)
                        .with_sub_index(quantize_distance(
                            last_occurrence_distance))
                        .with_sub_index(repeated_bit.to_u8() as usize),
                    |mixer| {
                        mixer.set_input(0, direct.0, direct.1);
                        mixer.set_input(1, edge_fixed.0, edge_fixed.1);
                    }, luts);
                mixing_result.0
            }
            &ContextState::ForNode {
                last_occurrence_distance, probability_estimator, bits_runs,
                ref cost_trackers, ..
            } => {
                let non_stationary_1_prediction = self.node_non_stationary_1
                    .predict(luts, |indexer| indexer
                        .with_sub_index(OccurrenceCountQuantizer::quantize(
                            bits_runs.last_bit_run_length()))
                        .with_sub_index(bits_runs.last_bit().to_u8() as usize));
                let non_stationary_3_prediction = self.node_non_stationary_3
                    .predict(luts, |indexer| indexer
                        .with_sub_index(OccurrenceCountQuantizer::quantize(
                            bits_runs.last_bit_previous_run_length()))
                        .with_sub_index(OccurrenceCountQuantizer::quantize(
                            bits_runs.opposite_bit_run_length()))
                        .with_sub_index(OccurrenceCountQuantizer::quantize(
                            bits_runs.last_bit_run_length()))
                        .with_sub_index(bits_runs.last_bit().to_u8() as usize));
                let stationary_prediction_sq =
                    probability_estimator.prediction();
                let stationary_prediction_st =
                    luts.stretch_lut().stretch(stationary_prediction_sq);
                let quantized_usage_count = OccurrenceCountQuantizer::quantize(
                    probability_estimator.usage_count());
                let mixing_result = self.node_mixer_kit.predict(
                    |indexer| indexer
                        .with_sub_index(quantize_cost_trackers(cost_trackers))
                        .with_sub_index(quantize_distance(
                            last_occurrence_distance))
                        .with_sub_index(quantized_usage_count),
                    |mixer| {
                        mixer.set_input(0, stationary_prediction_sq,
                                        stationary_prediction_st);
                        mixer.set_input(1, non_stationary_1_prediction.0,
                                        non_stationary_1_prediction.1);
                        mixer.set_input(2, non_stationary_3_prediction.0,
                                        non_stationary_3_prediction.1);
                    }, luts);
                mixing_result.0
            }
        }
    }

    pub fn update(&mut self, ctx_state: &ContextState, input_bit: Bit,
                  luts: &LookUpTables) -> Option<CostTrackers> {
        match ctx_state {
            &ContextState::ForEdge { .. } => {
                self.edge_mixer_kit.update(input_bit, 500, luts);
                None
            }
            &ContextState::ForNode { ref cost_trackers, .. } => {
                self.node_non_stationary_1.update(input_bit, luts);
                self.node_non_stationary_3.update(input_bit, luts);
                let new_cost_trackers = {
                    let mixer = self.node_mixer_kit.current_mixer();
                    cost_trackers.updated(
                        mixer.prediction_sq(0), mixer.prediction_sq(1),
                        input_bit, luts)
                };
                self.node_mixer_kit.update(input_bit, 100, luts);
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
