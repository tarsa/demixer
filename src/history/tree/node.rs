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
use core::fmt;

use bit::Bit;
use estimators::decelerating::DeceleratingEstimator;
use history::{ContextState, updated_bit_history};
use history::window::WindowIndex;
use lut::estimator::DeceleratingEstimatorLut;
use super::direction::Direction;
use super::node_child::{NodeChild, NodeChildren};

#[derive(Clone)]
pub struct Node {
    pub children: NodeChildren,
    pub text_start: u32,
    probability_estimator: DeceleratingEstimator,
    history_state: u16,
    depth: u16,
    left_count: u16,
    right_count: u16,
}

impl Node {
    pub const INVALID: Node = Node {
        children: NodeChildren::INVALID,
        text_start: 0,
        probability_estimator: DeceleratingEstimator::INVALID,
        history_state: 0,
        depth: 0,
        left_count: 0,
        right_count: 0,
    };

    pub fn new(text_start: WindowIndex,
               probability_estimator: DeceleratingEstimator, depth: usize,
               left_count: u16, right_count: u16, history_state: u16,
               children: NodeChildren) -> Node {
        assert!((text_start.raw() as u64) < 1u64 << 31);
        assert!((depth as u64) < 1u64 << 16);
        assert!((left_count as u64) < 1u64 << 16);
        assert!((right_count as u64) < 1u64 << 16);
        assert!((history_state as u64) < 1u64 << 16);
        Node {
            children,
            text_start: text_start.raw() as u32,
            probability_estimator,
            history_state,
            depth: depth as u16,
            left_count,
            right_count,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.children[Direction::Left] != NodeChild::INVALID &&
            self.children[Direction::Right] != NodeChild::INVALID
    }

    pub fn text_start(&self) -> WindowIndex {
        WindowIndex::new(self.text_start as usize)
    }

    pub fn probability_estimator(&self) -> DeceleratingEstimator {
        self.probability_estimator
    }

    pub fn depth(&self) -> usize {
        self.depth as usize
    }

    pub fn left_count(&self) -> u16 {
        self.left_count
    }

    pub fn right_count(&self) -> u16 {
        self.right_count
    }

    pub fn history_state(&self) -> u16 {
        self.history_state
    }

    pub fn child(&self, direction: Direction) -> NodeChild {
        self.children[direction]
    }

    pub fn increment_edge_counters(&mut self, direction: Direction,
                                   lut: &DeceleratingEstimatorLut) {
        match direction {
            Direction::Left => self.left_count =
                ContextState::MAX_OCCURRENCE_COUNT.min(self.left_count + 1),
            Direction::Right => self.right_count =
                ContextState::MAX_OCCURRENCE_COUNT.min(self.right_count + 1),
        }
        let bit: Bit = direction.into();
        self.history_state = updated_bit_history(self.history_state, bit);
        self.probability_estimator.update(bit, &lut);
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}'{}'{:b}'l({})r({})",
               self.text_start(), self.depth(), self.history_state(),
               self.left_count(), self.right_count())
    }
}
