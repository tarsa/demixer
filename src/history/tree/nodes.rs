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
use std::{fmt, ops};

use super::direction::Direction;
use super::node::Node;
use super::node_child::NodeChild;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeIndex {
    value: usize
}

impl NodeIndex {
    pub fn new(value: usize) -> NodeIndex {
        NodeIndex { value }
    }

    pub fn raw(&self) -> usize {
        self.value
    }

    pub fn is_root(&self) -> bool {
        self.value < Nodes::NUM_ROOTS
    }
}

impl fmt::Display for NodeIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

pub struct Nodes {
    items: Vec<Node>,
    last_deleted_node_idx_opt: Option<NodeIndex>,
    removed_nodes_count: usize,
}

impl Nodes {
    pub const NUM_ROOTS: usize = 1;

    pub fn new(nodes_limit: usize) -> Nodes {
        assert!(nodes_limit >= Nodes::NUM_ROOTS);
        let mut items = Vec::with_capacity(nodes_limit);
        (0..Nodes::NUM_ROOTS).for_each(|_| items.push(Node::INVALID));
        Nodes {
            items,
            last_deleted_node_idx_opt: None,
            removed_nodes_count: 0,
        }
    }

    pub fn add_node(&mut self, node: Node) -> NodeChild {
        if let Some(last_deleted_node_index) = self.last_deleted_node_idx_opt {
            assert!(self.removed_nodes_count > 0);
            self.removed_nodes_count -= 1;
            let old_node_children = self[last_deleted_node_index].children;
            assert!(!old_node_children[Direction::Left].is_valid());
            let next_deleted_node_handle = old_node_children[Direction::Right];
            if next_deleted_node_handle.is_valid() {
                assert!(next_deleted_node_handle.is_node_index());
                self.last_deleted_node_idx_opt =
                    Some(next_deleted_node_handle.to_node_index());
            } else {
                self.last_deleted_node_idx_opt = None;
            }
            self.update_node(last_deleted_node_index, node);
            last_deleted_node_index.into()
        } else {
            assert_eq!(self.removed_nodes_count, 0);
            assert!(self.items.capacity() > self.items.len());
            let node_child: NodeChild = NodeIndex::new(self.items.len()).into();
            self.items.push(node);
            node_child
        }
    }

    pub fn update_node(&mut self, node_index: NodeIndex, new_node: Node) {
        self.items[node_index.value] = new_node;
    }

    pub fn delete_node(&mut self, node_index: NodeIndex) {
        let mut node = Node::INVALID;
        node.children[Direction::Left] = NodeChild::INVALID;
        node.children[Direction::Right] = self.last_deleted_node_idx_opt
            .map(|node_index| node_index.into()).unwrap_or(NodeChild::INVALID);
        self[node_index] = node;
        self.last_deleted_node_idx_opt = Some(node_index);
        self.removed_nodes_count += 1;
    }

    pub fn live_nodes_count(&self) -> usize {
        if self.items[0].is_valid() {
            self.items.len() - self.removed_nodes_count
        } else {
            0
        }
    }
}

impl ops::Index<NodeIndex> for Nodes {
    type Output = Node;

    fn index(&self, node_index: NodeIndex) -> &Node {
        let index = node_index.value;
        let node = &self.items[index];
        assert!(index >= Nodes::NUM_ROOTS || node.is_valid());
        node
    }
}

impl ops::IndexMut<NodeIndex> for Nodes {
    fn index_mut(&mut self, node_index: NodeIndex) -> &mut Node {
        let index = node_index.value;
        let node = &mut self.items[index];
        assert!(index >= Nodes::NUM_ROOTS || node.is_valid());
        node
    }
}
