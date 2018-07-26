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
use estimators::cost::CostTracker;
use estimators::decelerating::DeceleratingEstimator;
use history::ContextState;
use history::state::bits_runs::BitsRunsTracker;
use history::state::recent_bits::RecentBitsHistory;
use history::window::WindowIndex;
use lut::estimator::DeceleratingEstimatorRates;
use super::direction::Direction;
use super::node_child::{NodeChild, NodeChildren};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostTrackers {
    stationary: CostTracker,
    non_stationary: CostTracker,
}

impl CostTrackers {
    pub const DEFAULT: Self = CostTrackers {
        stationary: CostTracker::INITIAL,
        non_stationary: CostTracker::INITIAL,
    };

    pub fn new(stationary: CostTracker, non_stationary: CostTracker) -> Self {
        CostTrackers { stationary, non_stationary }
    }

    pub fn stationary(&self) -> CostTracker { self.stationary }

    pub fn non_stationary(&self) -> CostTracker { self.non_stationary }
}

#[derive(Clone)]
pub struct Node {
    pub children: NodeChildren,
    pub text_start: u32,
    probability_estimator: DeceleratingEstimator,
    cost_trackers: CostTrackers,
    bits_runs: BitsRunsTracker,
    recent_bits: RecentBitsHistory,
    depth: u16,
    left_count: u16,
    right_count: u16,
}

impl Node {
    pub const INVALID: Node = Node {
        children: NodeChildren::INVALID,
        text_start: 0,
        probability_estimator: DeceleratingEstimator::INVALID,
        cost_trackers: CostTrackers::DEFAULT,
        bits_runs: BitsRunsTracker::NEW,
        recent_bits: RecentBitsHistory::INVALID,
        depth: 0,
        left_count: 0,
        right_count: 0,
    };

    pub fn new(text_start: WindowIndex,
               probability_estimator: DeceleratingEstimator, depth: usize,
               cost_trackers: CostTrackers, bits_runs: BitsRunsTracker,
               left_count: u16, right_count: u16,
               recent_bits: RecentBitsHistory, children: NodeChildren) -> Node {
        assert!((text_start.raw() as u64) < 1u64 << 31);
        assert!((depth as u64) < 1u64 << 16);
        assert!((left_count as u64) < 1u64 << 16);
        assert!((right_count as u64) < 1u64 << 16);
        assert!(recent_bits.is_valid());
        Node {
            children,
            text_start: text_start.raw() as u32,
            probability_estimator,
            cost_trackers,
            bits_runs,
            recent_bits,
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

    pub fn cost_trackers(&self) -> CostTrackers {
        self.cost_trackers.clone()
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

    pub fn bits_runs(&self) -> BitsRunsTracker {
        self.bits_runs
    }

    pub fn recent_bits(&self) -> RecentBitsHistory {
        self.recent_bits
    }

    pub fn child(&self, direction: Direction) -> NodeChild {
        self.children[direction]
    }

    pub fn update_on_next_bit(&mut self, bit: Bit,
                              new_cost_trackers: CostTrackers,
                              lut: &DeceleratingEstimatorRates) {
        let direction: Direction = bit.into();
        match direction {
            Direction::Left => self.left_count =
                ContextState::MAX_OCCURRENCE_COUNT.min(self.left_count + 1),
            Direction::Right => self.right_count =
                ContextState::MAX_OCCURRENCE_COUNT.min(self.right_count + 1),
        }
        self.bits_runs = self.bits_runs.updated(bit);
        self.recent_bits = self.recent_bits.updated(bit);
        self.probability_estimator.update(bit, &lut);
        self.cost_trackers = new_cost_trackers;
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}'{}'{:b}'l({})r({})",
               self.text_start(), self.depth(),
               self.recent_bits().last_7_bits(),
               self.left_count(), self.right_count())
    }
}
