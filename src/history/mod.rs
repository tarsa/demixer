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
use self::window::WindowIndex;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextState {
    ForEdge {
        last_occurrence_index: WindowIndex,
        occurrence_count: u16,
        repeated_bit: Bit,
    },
    ForNode {
        last_occurrence_index: WindowIndex,
        probability_estimator: DeceleratingEstimator,
        bit_history: TheHistoryState,
    },
}

impl ContextState {
    pub const MAX_OCCURRENCE_COUNT: u16 =
        DeceleratingEstimator::MAX_COUNT;

    pub fn last_occurrence_index(&self) -> WindowIndex {
        match self {
            &ContextState::ForEdge { last_occurrence_index, .. } =>
                last_occurrence_index,
            &ContextState::ForNode { last_occurrence_index, .. } =>
                last_occurrence_index,
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

    fn starting_state(first_occurrence_index: WindowIndex, bit_in_context: Bit)
                      -> ContextState {
        ContextState::ForEdge {
            last_occurrence_index: first_occurrence_index,
            occurrence_count: 1,
            repeated_bit: bit_in_context,
        }
    }

    fn next_state(&self, new_occurrence_index: WindowIndex, bit_in_context: Bit,
                  luts: &LookUpTables) -> ContextState {
        match self {
            &ContextState::ForNode {
                probability_estimator, bit_history, ..
            } => {
                let mut probability_estimator =
                    probability_estimator.clone();
                probability_estimator.update(
                    bit_in_context, luts.d_estimator_lut());
                ContextState::ForNode {
                    last_occurrence_index: new_occurrence_index,
                    probability_estimator,
                    bit_history: bit_history.updated(bit_in_context),
                }
            }
            &ContextState::ForEdge {
                occurrence_count, repeated_bit, ..
            } => {
                if repeated_bit == bit_in_context {
                    ContextState::ForEdge {
                        last_occurrence_index: new_occurrence_index,
                        occurrence_count:
                        ContextState::MAX_OCCURRENCE_COUNT
                            .min(occurrence_count + 1),
                        repeated_bit,
                    }
                } else {
                    let mut d_estimator =
                        luts.d_estimator_cache().for_bit_run(
                            repeated_bit, occurrence_count);
                    d_estimator.update(
                        bit_in_context, luts.d_estimator_lut());
                    let bit_history = luts.history_state_factory().for_new_node(
                        bit_in_context, occurrence_count);
                    ContextState::ForNode {
                        last_occurrence_index: new_occurrence_index,
                        probability_estimator: d_estimator,
                        bit_history,
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

    fn process_input_bit(&mut self, input_bit: Bit);
}
