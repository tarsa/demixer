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
pub mod naive;
pub mod fat_map;
pub mod state;
pub mod tree;
pub mod window;

use bit::Bit;
use estimators::decelerating::DeceleratingEstimator;
use lut::LookUpTables;
use self::state::{TheHistoryState, HistoryState, HistoryStateFactory};
use self::tree::node::CostTrackers;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextState {
    ForEdge {
        last_occurrence_distance: usize,
        occurrence_count: u16,
        repeated_bit: Bit,
    },
    ForNode {
        last_occurrence_distance: usize,
        probability_estimator: DeceleratingEstimator,
        bit_history: TheHistoryState,
        cost_trackers: CostTrackers,
    },
}

impl ContextState {
    pub const MAX_OCCURRENCE_COUNT: u16 =
        DeceleratingEstimator::MAX_COUNT;

    pub fn is_for_node(&self) -> bool {
        match self {
            &ContextState::ForEdge { .. } => false,
            &ContextState::ForNode { .. } => true,
        }
    }

    pub fn last_occurrence_distance(&self) -> usize {
        match self {
            &ContextState::ForEdge { last_occurrence_distance, .. } =>
                last_occurrence_distance,
            &ContextState::ForNode { last_occurrence_distance, .. } =>
                last_occurrence_distance,
        }
    }

    pub fn occurrence_count(&self) -> u16 {
        match self {
            &ContextState::ForEdge { occurrence_count, .. } =>
                occurrence_count,
            &ContextState::ForNode { probability_estimator, .. } =>
                probability_estimator.usage_count(),
        }
    }

    pub fn bit_history(&self, luts: &LookUpTables) -> TheHistoryState {
        match self {
            &ContextState::ForEdge { occurrence_count, repeated_bit, .. } =>
                luts.history_state_factory()
                    .for_bit_run(repeated_bit, occurrence_count),
            &ContextState::ForNode { bit_history, .. } =>
                bit_history,
        }
    }

    fn starting_state(first_occurrence_distance: usize, bit_in_context: Bit)
                      -> ContextState {
        ContextState::ForEdge {
            last_occurrence_distance: first_occurrence_distance,
            occurrence_count: 1,
            repeated_bit: bit_in_context,
        }
    }

    pub fn neutralize_cost_trackers(&mut self) {
        match self {
            &mut ContextState::ForEdge { .. } => (),
            &mut ContextState::ForNode { ref mut cost_trackers, .. } =>
                *cost_trackers = CostTrackers::DEFAULT,
        }
    }

    fn next_state(&self, new_occurrence_distance: usize,
                  bit_in_context: Bit, cost_trackers_opt: Option<CostTrackers>,
                  luts: &LookUpTables) -> ContextState {
        match self {
            &ContextState::ForNode {
                probability_estimator, bit_history, ..
            } => {
                assert!(cost_trackers_opt.is_some());
                let mut probability_estimator =
                    probability_estimator.clone();
                probability_estimator.update(
                    bit_in_context, luts.d_estimator_rates());
                ContextState::ForNode {
                    last_occurrence_distance: new_occurrence_distance,
                    probability_estimator,
                    bit_history: bit_history.updated(bit_in_context),
                    cost_trackers: cost_trackers_opt.unwrap(),
                }
            }
            &ContextState::ForEdge {
                occurrence_count, repeated_bit, ..
            } => {
                assert!(cost_trackers_opt.is_none());
                if repeated_bit == bit_in_context {
                    ContextState::ForEdge {
                        last_occurrence_distance: new_occurrence_distance,
                        occurrence_count:
                        ContextState::MAX_OCCURRENCE_COUNT
                            .min(occurrence_count + 1),
                        repeated_bit,
                    }
                } else {
                    let d_estimator = luts.d_estimator_cache().for_new_node(
                        bit_in_context, occurrence_count);
                    let bit_history = luts.history_state_factory().for_new_node(
                        bit_in_context, occurrence_count);
                    let cost_tracker =
                        luts.cost_trackers_lut().for_new_node(occurrence_count);
                    let cost_trackers =
                        CostTrackers::new(cost_tracker, cost_tracker);
                    ContextState::ForNode {
                        last_occurrence_distance: new_occurrence_distance,
                        probability_estimator: d_estimator,
                        bit_history,
                        cost_trackers,
                    }
                }
            }
        }
    }
}

pub struct CollectedContextStates {
    items: Vec<ContextState>,
}

impl CollectedContextStates {
    pub fn new(max_order: usize) -> CollectedContextStates {
        CollectedContextStates {
            items: Vec::with_capacity(max_order + 1)
        }
    }

    pub fn items(&self) -> &[ContextState] {
        &self.items
    }

    pub fn push(&mut self, context_state: ContextState) {
        assert_ne!(self.items.len(), self.items.capacity());
        self.items.push(context_state);
    }

    pub fn reset(&mut self) {
        self.items.clear();
    }
}

pub trait HistorySource<'a> {
    fn new(max_window_size: usize, max_order: usize, luts: &'a LookUpTables)
           -> Self;

    fn start_new_byte(&mut self);

    fn gather_history_states(
        &self, context_states: &mut CollectedContextStates);

    fn process_input_bit(&mut self, input_bit: Bit,
                         new_cost_trackers: &[CostTrackers]);
}
