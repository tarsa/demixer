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
use ::history::tree::nodes::Nodes;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeChild {
    index: i32
}

impl NodeChild {
    // root node can't be a child
    pub const INVALID: NodeChild = NodeChild { index: !0 };

    pub fn from_window_index(window_index: usize) -> NodeChild {
        assert!(window_index <= 0x7fff_ffff);
        NodeChild { index: window_index as i32 }
    }

    pub fn from_node_index(node_index: usize) -> NodeChild {
        assert!(node_index >= Nodes::NUM_ROOTS && node_index <= 0x7fff_ffff);
        NodeChild { index: !(node_index as i32) }
    }

    pub fn is_valid(&self) -> bool {
        self.index >= 0 || (!self.index) as usize >= Nodes::NUM_ROOTS
    }

    pub fn is_window_index(&self) -> bool {
        self.index >= 0
    }

    pub fn is_node_index(&self) -> bool {
        self.index < 0
    }

    pub fn to_window_index(&self) -> WindowIndex {
        WindowIndex::new(self.index)
    }

    pub fn to_node_index(&self) -> NodeIndex {
        NodeIndex::new(!self.index)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeIndex {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowIndex {
    pub index: usize
}

impl WindowIndex {
    pub fn new(index: i32) -> WindowIndex {
        assert!(index >= 0);
        WindowIndex { index: index as usize }
    }
}
