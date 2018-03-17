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

use ::history::updated_bit_history;
use ::history::tree::direction::Direction;
use ::history::tree::node_child::NodeChild;
use ::history::tree::window::WindowIndex;

#[derive(Clone)]
pub struct Node {
    pub children: [NodeChild; 2],
    // counter: SimpleCounter,
    pub text_start: u32,
    history_state: u16,
    pub depth: u16,
    left_count: u16,
    right_count: u16,
}

impl Node {
    pub const INVALID: Node = Node {
        children: [NodeChild::INVALID, NodeChild::INVALID],
        text_start: 0,
        history_state: 0,
        depth: 0,
        left_count: 0,
        right_count: 0,
    };

    pub fn new(text_start: WindowIndex, depth: usize,
               left_count: usize, right_count: usize, history_state: u32,
               children: [NodeChild; 2]) -> Node {
        assert!((text_start.raw() as u64) < 1u64 << 31);
        assert!((depth as u64) < 1u64 << 16);
        assert!((left_count as u64) < 1u64 << 16);
        assert!((right_count as u64) < 1u64 << 16);
        assert!((history_state as u64) < 1u64 << 16);
        Node {
            children,
            text_start: text_start.raw() as u32,
            history_state: history_state as u16,
            depth: depth as u16,
            left_count: left_count as u16,
            right_count: right_count as u16,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.children[0] != NodeChild::INVALID &&
            self.children[1] != NodeChild::INVALID
    }

    pub fn text_start(&self) -> WindowIndex {
        WindowIndex::new(self.text_start as usize)
    }

    pub fn depth(&self) -> usize {
        self.depth as usize
    }

    pub fn left_count(&self) -> usize {
        self.left_count as usize
    }

    pub fn right_count(&self) -> usize {
        self.right_count as usize
    }

    pub fn history_state(&self) -> u32 {
        self.history_state as u32
    }

    pub fn child(&self, direction: Direction) -> NodeChild {
        self.children[direction]
    }

    pub fn increment_edge_counters(&mut self, direction: Direction) {
        match direction {
            Direction::Left =>
                self.left_count = 63.min(self.left_count + 1),
            Direction::Right =>
                self.right_count = 63.min(self.right_count + 1),
        }
        self.history_state = updated_bit_history(
            self.history_state(), direction.fold(|| false, || true)) as u16;
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}'{}'{:b}'l({})r({})",
               self.text_start(), self.depth(), self.history_state(),
               self.left_count(), self.right_count())
    }
}
