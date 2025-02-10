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
    MixerInitializationMode, Mixer, FixedSizeMixer, Mixer3, Mixer4,
};
use util::indexer::{Indexer, Indexer2};
use util::quantizers::OccurrenceCountQuantizer;
use super::kits::{EstimatorsWithIndexer, MixersWithIndexer};

pub struct SingleContextPredictor {
    edge_fixed_prediction: (FractOnlyU32, StretchedProbD),
    edge_occur_and_byte: EstimatorsWithIndexer<Indexer2>,
    node_bits_run: EstimatorsWithIndexer<Indexer2>,
}

impl SingleContextPredictor {
    pub fn new_edge_mixer() -> Mixer4 {
        Mixer4::new(15, MixerInitializationMode::DominantFirst)
    }
    pub fn edge_mixer_extra_dimensions() -> Vec<usize> {
        vec![4, 2]
    }

    pub fn new_node_mixer() -> Mixer3 {
        Mixer3::new(15, MixerInitializationMode::DominantFirst)
    }
    pub fn node_mixer_extra_dimensions() -> Vec<usize> {
        vec![6, 4, OccurrenceCountQuantizer::max_output() + 1]
    }

    pub fn new(luts: &LookUpTables) -> Self {
        let max_quantized_bit_run_length = OccurrenceCountQuantizer::quantize(
            BitsRunsTracker::MAX_RUN_LENGTH);
        let edge_fixed_st = StretchedProbD::new(2 << 21, 21);
        let edge_fixed_sq = luts.squash_lut().squash(edge_fixed_st);
        SingleContextPredictor {
            edge_fixed_prediction: (edge_fixed_sq, edge_fixed_st),
            edge_occur_and_byte: EstimatorsWithIndexer::new(
                Indexer2::new(vec![max_quantized_bit_run_length + 1, 256])),
            node_bits_run: EstimatorsWithIndexer::new(
                Indexer2::new(vec![
                    max_quantized_bit_run_length + 1, 2])),
        }
    }

    pub fn predict<Idx: Indexer, Mxr: Mixer>(
        &mut self, mixer_kit: &mut MixersWithIndexer<Mxr, Idx>,
        ctx_state: &ContextState, context_byte: u8,
        lower_order_probability: (FractOnlyU32, StretchedProbD),
        luts: &LookUpTables) -> (FractOnlyU32, StretchedProbD) {
        let lower = lower_order_probability;
        match ctx_state {
            &ContextState::ForEdge {
                repeated_bit, occurrence_count, last_occurrence_distance
            } => {
                let quantized_count = OccurrenceCountQuantizer::quantize(
                    occurrence_count);
                let quantized_distance = quantize_distance(
                    last_occurrence_distance);
                let occur_and_byte =
                    self.edge_occur_and_byte.predict(luts, |indexer| indexer
                        .with_sub_index(quantized_count)
                        .with_sub_index(context_byte as usize));
                let direct = luts.direct_predictions()
                    .for_0_bit_run(occurrence_count);
                let edge_fixed = self.edge_fixed_prediction;
                let mixing_result = mixer_kit.predict(
                    |indexer| indexer
                        .with_sub_index(quantized_distance)
                        .with_sub_index(repeated_bit.to_u8() as usize),
                    |mixer| {
                        mixer.set_input(0, lower.0, lower.1);
                        mixer.set_input(1, direct.0, direct.1);
                        mixer.set_input(2, edge_fixed.0, edge_fixed.1);
                        mixer.set_input(3, occur_and_byte.0, occur_and_byte.1);
                    }, luts);
                mixing_result
            }
            &ContextState::ForNode {
                last_occurrence_distance, probability_estimator, bits_runs,
                ref cost_trackers, ..
            } => {
                let stationary_prediction_sq =
                    probability_estimator.prediction();
                let stationary_prediction_st =
                    luts.stretch_lut().stretch(stationary_prediction_sq);
                let bits_runs_1_prediction = self.node_bits_run
                    .predict(luts, |indexer| indexer
                        .with_sub_index(OccurrenceCountQuantizer::quantize(
                            bits_runs.last_bit_run_length()))
                        .with_sub_index(bits_runs.last_bit().to_u8() as usize));
                let quantized_run_length = OccurrenceCountQuantizer::quantize(
                    bits_runs.opposite_bit_run_length());
                let mixing_result = mixer_kit.predict(
                    |indexer| indexer
                        .with_sub_index(quantize_cost_trackers(cost_trackers))
                        .with_sub_index(quantize_distance(
                            last_occurrence_distance))
                        .with_sub_index(quantized_run_length),
                    |mixer| {
                        mixer.set_input(0, lower.0, lower.1);
                        mixer.set_input(1, stationary_prediction_sq,
                                        stationary_prediction_st);
                        mixer.set_input(2, bits_runs_1_prediction.0,
                                        bits_runs_1_prediction.1);
                    }, luts);
                mixing_result
            }
        }
    }

    pub fn update<Idx: Indexer, Mxr: Mixer>(
        &mut self, mixer_kit: &mut MixersWithIndexer<Mxr, Idx>,
        ctx_state: &ContextState, input_bit: Bit,
        luts: &LookUpTables) -> Option<CostTrackers> {
        match ctx_state {
            &ContextState::ForEdge { .. } => {
                self.edge_occur_and_byte.update(input_bit, luts);
                mixer_kit.update(input_bit, 700, luts);
                None
            }
            &ContextState::ForNode { ref cost_trackers, .. } => {
                self.node_bits_run.update(input_bit, luts);
                let new_cost_trackers = {
                    let mixer = mixer_kit.current_mixer();
                    cost_trackers.updated(
                        mixer.prediction_sq(1), mixer.prediction_sq(2),
                        input_bit, luts)
                };
                mixer_kit.update(input_bit, 250, luts);
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
