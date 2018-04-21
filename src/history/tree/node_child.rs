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
use std::ops;

use history::window::WindowIndex;
use super::direction::Direction;
use super::nodes::{NodeIndex, Nodes};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeChild {
    value: i32
}

impl NodeChild {
    // root node can't be a child
    pub const INVALID: NodeChild = NodeChild { value: !0 };

    pub fn is_valid(&self) -> bool {
        self.value >= 0 || (!self.value) as usize >= Nodes::NUM_ROOTS
    }

    pub fn is_window_index(&self) -> bool {
        self.value >= 0
    }

    pub fn is_node_index(&self) -> bool {
        self.value < 0
    }

    pub fn to_window_index(&self) -> WindowIndex {
        assert!(self.value >= 0);
        WindowIndex::new(self.value as usize)
    }

    pub fn to_node_index(&self) -> NodeIndex {
        assert!(self.value < 0);
        NodeIndex::new(!self.value as usize)
    }
}

impl From<WindowIndex> for NodeChild {
    fn from(window_index: WindowIndex) -> NodeChild {
        assert!(window_index.raw() <= 0x7fff_ffff);
        NodeChild { value: window_index.raw() as i32 }
    }
}

impl From<NodeIndex> for NodeChild {
    fn from(node_index: NodeIndex) -> NodeChild {
        let node_index = node_index.raw();
        assert!(node_index >= Nodes::NUM_ROOTS && node_index <= 0x7fff_ffff);
        NodeChild { value: !(node_index as i32) }
    }
}

#[derive(Clone)]
pub struct NodeChildren([NodeChild; 2]);

impl NodeChildren {
    pub const INVALID: Self =
        NodeChildren([NodeChild::INVALID, NodeChild::INVALID]);

    pub fn new(children: [NodeChild; 2]) -> Self {
        NodeChildren(children)
    }

    pub fn items(&self) -> &[NodeChild; 2] {
        &self.0
    }

    fn items_mut(&mut self) -> &mut [NodeChild; 2] {
        &mut self.0
    }
}

impl ops::Index<Direction> for NodeChildren {
    type Output = NodeChild;

    fn index(&self, index: Direction) -> &NodeChild {
        match index {
            Direction::Left => &self.items()[0],
            Direction::Right => &self.items()[1],
        }
    }
}

impl ops::IndexMut<Direction> for NodeChildren {
    fn index_mut(&mut self, index: Direction) -> &mut NodeChild {
        match index {
            Direction::Left => &mut self.items_mut()[0],
            Direction::Right => &mut self.items_mut()[1],
        }
    }
}
