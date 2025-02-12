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
use std::collections::HashMap;
use std::ops;

use PRINT_DEBUG;
use history::ContextState;
use history::window::WindowIndex;
use lut::estimator::DeceleratingEstimatorRates;
use super::{Tree, TreeState};
use super::direction::Direction;
use super::node::CostTrackers;
use super::nodes::NodeIndex;


#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Context {
    pub suffix_index: WindowIndex,
    pub node_index: NodeIndex,
    pub incoming_edge_visits_count: i32,
    pub in_leaf: bool,
    pub direction_from_parent: Option<Direction>,
}

impl Context {
    pub fn descend(&mut self, tree: &mut Tree, order: usize, bit_index: usize,
                   new_cost_trackers: CostTrackers,
                   rates_lut: &DeceleratingEstimatorRates) {
        assert!(!self.in_leaf);
        let bit = tree.window.get_bit(tree.window.cursor(), bit_index);
        let direction: Direction = bit.into();
        self.direction_from_parent = Some(direction);
        let node_index = self.node_index;
        self.incoming_edge_visits_count = {
            let node = &tree.nodes()[node_index];
            if direction == Direction::Left {
                node.left_count()
            } else {
                node.right_count()
            }
        } as i32;
        tree.nodes_mut()[node_index].update_on_next_bit(
            bit, new_cost_trackers, rates_lut);
        let child = tree.nodes()[node_index].child(direction);
        if child.is_window_index() {
            self.in_leaf = true;
            self.suffix_index = child.to_window_index();
            tree.nodes_mut()[node_index].children[direction] =
                tree.window.index_subtract(tree.window.cursor(), order).into();
        } else {
            self.node_index = child.to_node_index();
            self.suffix_index = tree.nodes()[self.node_index].text_start();
            tree.nodes_mut()[self.node_index].text_start =
                tree.window.index_subtract(tree.window.cursor(),
                                           order).raw() as u32;
        }
        if PRINT_DEBUG {
            println!("DESCEND, order = {}, after = {}", order, self);
        }
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "sfx:{},node:{},incnt:{},inleaf:{},dir:{}",
               self.suffix_index,
               self.node_index,
               self.incoming_edge_visits_count,
               self.in_leaf,
               self.direction_from_parent.map(|dir|
                   dir.fold(|| "left", || "right")).unwrap_or("none"))
    }
}

#[derive(Debug)]
pub struct ActiveContexts {
    pub items: Vec<Context>,
}

impl ActiveContexts {
    pub fn new(max_order: usize) -> ActiveContexts {
        ActiveContexts {
            items: Vec::with_capacity(max_order + 1),
        }
    }

    pub fn shift(&mut self, tree: &mut Tree) {
        if tree.tree_state() == TreeState::Degenerate {
            assert_eq!(self.count(), 0);
            return;
        }
        if self.max_order() + 1 == self.items.len() {
            self.items.pop().unwrap();
        }
        let root_index = tree.get_root_node_index();
        tree.nodes[root_index].text_start = tree.window.cursor().raw() as u32;
        let root = &tree.nodes[root_index];
        let incoming_edge_visits_count = ContextState::MAX_OCCURRENCE_COUNT
            .min(root.left_count() + root.right_count()) as i32;
        let suffix_index = tree.window.index_decrement(tree.window.cursor());
        self.items.insert(0, Context {
            suffix_index,
            node_index: root_index,
            in_leaf: false,
            incoming_edge_visits_count,
            direction_from_parent: None,
        });
    }

    pub fn max_order(&self) -> usize {
        self.items.capacity() - 1
    }

    pub fn count(&self) -> usize {
        self.items.len()
    }

    pub fn keep_only(&mut self, count: usize) {
        self.items.split_off(count);
    }

    pub fn items(&self) -> &[Context] {
        &self.items
    }

    pub fn check_integrity_on_next_byte(&self, tree: &Tree) {
        if tree.tree_state() == TreeState::Proper {
            let mut contexts_suffixes_map = HashMap::new();
            for ctx in self.items.iter() {
                contexts_suffixes_map.insert(ctx.node_index.raw(),
                                             ctx.suffix_index);
            }
            let mut stack = Vec::new();
            stack.push(tree.get_root_node_index());
            while let Some(node_index) = stack.pop() {
                let node = &tree.nodes[node_index];
                let node_text_start = *contexts_suffixes_map
                    .get(&node_index.raw()).unwrap_or(&node.text_start());
                let full_byte_length = node.depth() / 8;
                let bit_index = 7 - (node.depth() % 8);
                let children = &tree.nodes[node_index].children;
                for child in children.items().iter() {
                    assert!(child.is_valid());
                    if child.is_node_index() {
                        let child_node = &tree.nodes[child.to_node_index()];
                        assert!(tree.window.compare_for_equal_prefix(
                            node_text_start, child_node.text_start(),
                            bit_index, full_byte_length));
                        stack.push(child.to_node_index());
                    } else {
                        assert!(tree.window.compare_for_equal_prefix(
                            node_text_start, child.to_window_index(),
                            bit_index, full_byte_length));
                    }
                }
            }
        }
    }
}

impl ops::Index<usize> for ActiveContexts {
    type Output = Context;

    fn index(&self, index: usize) -> &Context {
        &self.items[index]
    }
}

impl ops::IndexMut<usize> for ActiveContexts {
    fn index_mut(&mut self, index: usize) -> &mut Context {
        &mut self.items[index]
    }
}

impl fmt::Display for ActiveContexts {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("active contexts: [")?;
        if let Some(head) = self.items.first() {
            head.fmt(f)?;
        }
        for item in self.items.iter().skip(1) {
            write!(f, "  {}", item)?;
        }
        f.write_str("]")
    }
}
