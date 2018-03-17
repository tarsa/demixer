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
use history::window::WindowIndex;
use super::nodes::Nodes;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeChild {
    value: i32
}

impl NodeChild {
    // root node can't be a child
    pub const INVALID: NodeChild = NodeChild { value: !0 };

    pub fn from_node_index(node_index: usize) -> NodeChild {
        assert!(node_index >= Nodes::NUM_ROOTS && node_index <= 0x7fff_ffff);
        NodeChild { value: !(node_index as i32) }
    }

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
        NodeIndex::new(!self.value)
    }
}

impl From<WindowIndex> for NodeChild {
    fn from(window_index: WindowIndex) -> NodeChild {
        assert!(window_index.raw() <= 0x7fff_ffff);
        NodeChild { value: window_index.raw() as i32 }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeIndex {
    // TODO encapsulate
    pub index: usize
}

impl NodeIndex {
    pub fn new(index: i32) -> NodeIndex {
        assert!(index >= 0);
        NodeIndex { index: index as usize }
    }

    pub fn is_root(&self) -> bool {
        self.index < Nodes::NUM_ROOTS
    }
}
