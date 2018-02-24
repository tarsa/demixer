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
extern crate core;

use core::fmt;
use std::ops;
use std::collections::HashMap;

use ::PRINT_DEBUG;
use history::{
    HistorySource,
    ContextState,
    CollectedContextStates,
    make_bit_run_history, updated_bit_history, get_bit, bytes_differ_on,
    compare_for_equal_prefix,
};

// TODO remove over-provisioning and replace with full window sliding support
const OVER_PROVISIONING_FACTOR: usize = 10;
const OVER_PROVISIONING_CONSTANT: usize = 100;

pub struct TreeHistorySource {
    pub tree: Tree,
    pub active_contexts: ActiveContexts,
    bit_index: usize,
}

impl HistorySource for TreeHistorySource {
    fn new(max_window_size: usize, max_order: usize) -> TreeHistorySource {
        assert!(max_window_size > 0);
        let nodes = Nodes::new(Nodes::NUM_ROOTS.max(max_window_size - 1));
        TreeHistorySource {
            tree: Tree::new(nodes, max_window_size, 0),
            active_contexts: ActiveContexts::new(max_order),
            bit_index: 7,
        }
    }

    fn start_new_byte(&mut self) {
        assert_eq!(self.bit_index, 7);
        self.active_contexts.shift(&mut self.tree);
        self.tree.start_new_byte(&mut self.active_contexts);
    }

    fn gather_history_states(&self,
                             bit_histories: &mut CollectedContextStates) {
        self.tree.gather_states(&self.active_contexts, bit_histories,
                                self.bit_index);
    }

    fn process_input_bit(&mut self, input_bit: bool) {
        let max_order = self.active_contexts.max_order();
        self.tree.extend(&mut self.active_contexts, input_bit, self.bit_index,
                         max_order);
        if self.bit_index > 0 {
            self.bit_index -= 1;
        } else {
            self.bit_index = 7;
            self.tree.window_cursor += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Left = 0,
    Right = 1,
}

impl Direction {
    fn fold<T, FL: FnOnce() -> T, FR: FnOnce() -> T>(
        &self, on_left: FL, on_right: FR) -> T {
        match *self {
            Direction::Left => on_left(),
            Direction::Right => on_right(),
        }
    }
}

impl ops::Index<Direction> for [NodeChild; 2] {
    type Output = NodeChild;

    fn index(&self, index: Direction) -> &NodeChild {
        match index {
            Direction::Left => &self[0],
            Direction::Right => &self[1],
        }
    }
}

impl ops::IndexMut<Direction> for [NodeChild; 2] {
    fn index_mut(&mut self, index: Direction) -> &mut NodeChild {
        match index {
            Direction::Left => &mut self[0],
            Direction::Right => &mut self[1],
        }
    }
}

impl From<bool> for Direction {
    fn from(bit: bool) -> Direction {
        match bit {
            false => Direction::Left,
            true => Direction::Right,
        }
    }
}

impl ops::Not for Direction {
    type Output = Direction;

    fn not(self) -> Direction {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TreeState {
    /** Every inner node has two leaves */
    Proper,
    /** Has only invalid root node, happens when all symbols are identical */
    Degenerate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Context {
    suffix_index: WindowIndex,
    node_index: NodeIndex,
    incoming_edge_visits_count: i32,
    in_leaf: bool,
    direction_from_parent: Option<Direction>,
}

impl Context {
    fn descend(&mut self, tree: &mut Tree, order: usize, bit_index: usize) {
        assert!(!self.in_leaf);
        let direction: Direction =
            get_bit(tree.window[tree.window_cursor], bit_index).into();
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
        tree.nodes_mut()[node_index].increment_edge_counters(direction);
        let child = tree.nodes()[node_index].child(direction);
        if child.is_window_index() {
            self.in_leaf = true;
            self.suffix_index = child.to_window_index();
            tree.nodes_mut()[node_index].children[direction] =
                NodeChild::from_window_index(tree.window_cursor - order);
        } else {
            self.node_index = child.to_node_index();
            self.suffix_index = WindowIndex::new(
                tree.nodes()[self.node_index].text_start as i32);
            tree.nodes_mut()[self.node_index].text_start =
                (tree.window_cursor - order) as u32;
        }
        if PRINT_DEBUG {
            println!("DESCEND, order = {}, after = {}", order, self);
        }
    }

    pub fn prepare_for_test(&self, offset: usize) -> Context {
        Context {
            suffix_index: WindowIndex {
                index: self.suffix_index.index - offset
            },
            node_index: NodeIndex::new(<i32>::max_value()),
            incoming_edge_visits_count: 0,
            ..self.clone()
        }
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "sfx:{},node:{},incnt:{},inleaf:{},dir:{}",
               self.suffix_index.index,
               self.node_index.index,
               self.incoming_edge_visits_count,
               self.in_leaf,
               self.direction_from_parent.map(|dir|
                   dir.fold(|| "left", || "right")).unwrap_or("none"))
    }
}

#[derive(Debug)]
pub struct ActiveContexts {
    items: Vec<Context>,
}

impl ActiveContexts {
    pub fn new(max_order: usize) -> ActiveContexts {
        ActiveContexts {
            items: Vec::with_capacity(max_order + 1),
        }
    }

    pub fn shift(&mut self, tree: &mut Tree) {
        if tree.tree_state == TreeState::Degenerate {
            assert_eq!(self.count(), 0);
            return;
        }
        if self.max_order() + 1 == self.items.len() {
            self.items.pop().unwrap();
        }
        let root_index = tree.get_root_node_index();
        tree.nodes[root_index].text_start = tree.window_cursor as u32;
        let root = &tree.nodes[root_index];
        let incoming_edge_visits_count =
            63.min(root.left_count() + root.right_count()) as i32;
        self.items.insert(0, Context {
            suffix_index: WindowIndex::new((tree.window_cursor - 1) as i32),
            node_index: root_index,
            in_leaf: false,
            incoming_edge_visits_count,
            direction_from_parent: None,
        });
    }

    fn max_order(&self) -> usize {
        self.items.capacity() - 1
    }

    fn count(&self) -> usize {
        self.items.len()
    }

    fn keep_only(&mut self, count: usize) {
        self.items.split_off(count);
    }

    pub fn items(&self) -> &[Context] {
        &self.items
    }

    pub fn check_integrity(&self, tree: &Tree) {
        if tree.tree_state == TreeState::Proper {
            let mut contexts_suffixes_map = HashMap::new();
            for ctx in self.items.iter() {
                contexts_suffixes_map.insert(ctx.node_index.index,
                                             ctx.suffix_index.index);
            }
            let mut stack = Vec::new();
            stack.push(tree.get_root_node_index());
            while let Some(node_index) = stack.pop() {
                let node = tree.nodes[node_index];
                let node_text_start = *contexts_suffixes_map
                    .get(&node_index.index).unwrap_or(&node.text_start());
                let full_byte_length = (node.depth / 8) as usize;
                let bit_index = 7 - (node.depth % 8) as usize;
                let children = tree.nodes.items[node_index.index].children;
                for child in children.iter() {
                    assert!(child.is_valid());
                    if child.is_node_index() {
                        let child_node = tree.nodes[child.to_node_index()];
                        assert!(compare_for_equal_prefix(
                            &tree.window, node_text_start,
                            child_node.text_start(),
                            bit_index, full_byte_length));
                        stack.push(child.to_node_index());
                    } else {
                        assert!(compare_for_equal_prefix(
                            &tree.window, node_text_start,
                            child.to_window_index().index,
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

pub struct Tree {
    nodes: Nodes,
    window: Vec<u8>,
    window_start: usize,
    pub window_cursor: usize,
    pub window_size: usize,
    max_window_size: usize,
    pub tree_state: TreeState,
    root_index: i32,
}

impl Tree {
    fn start_new_byte(&mut self, active_contexts: &mut ActiveContexts) {
        if self.window_size == self.max_window_size {
            assert_eq!(self.window_start + self.max_window_size,
                       self.window_cursor);
            assert_eq!(self.window_cursor, self.window.len());
            self.remove_leftmost_suffix(active_contexts);
        } else {
            assert!(self.window_size < self.max_window_size);
        }
        self.window.push(0);
        self.window_size += 1;
    }

    pub fn remove_leftmost_suffix(&mut self,
                                  active_contexts: &mut ActiveContexts) {
        if self.tree_state == TreeState::Degenerate {
            self.window[self.window_start] = 0;
            self.window_start += 1;
            self.window_size -= 1;
            return;
        }
        let mut parent_node_index_opt = None;
        let mut node_direction_opt = None;
        let mut node_index = self.get_root_node_index();
        let mut leaf_direction_opt = None;
        let mut leaf_sibling_opt = None;
        while leaf_direction_opt == None {
            let depth = self.nodes[node_index].depth();
            let direction: Direction = get_bit(
                self.window[self.window_start + depth / 8], 7 - (depth % 8),
            ).into();
            let child = self.nodes[node_index].child(direction);
            if child.is_node_index() {
                parent_node_index_opt = Some(node_index);
                node_direction_opt = Some(direction);
                node_index = child.to_node_index();
            } else {
                leaf_direction_opt = Some(direction);
                leaf_sibling_opt =
                    Some(self.nodes[node_index].child(!direction));
            }
        }
        let leaf_direction = leaf_direction_opt.unwrap();
        let leaf_sibling = leaf_sibling_opt.unwrap();
        let leaf_window_index = self.nodes[node_index].child(leaf_direction)
            .to_window_index();
        let node_found_in_active_contexts = active_contexts.items.iter()
            .find(|ctx| ctx.node_index.index == node_index.index).is_some();
        if PRINT_DEBUG { print!("DELETING: "); }
        if leaf_window_index.index > self.window_start {
            if PRINT_DEBUG {
                println!("skipped because prefix was repeated");
                println!("window start = {}, active contexts = {}",
                         self.window_start, active_contexts);
            }
            let mut new_active_contexts_count = active_contexts.count();
            for (order, ctx) in active_contexts.items.iter().enumerate().rev() {
                assert!(ctx.suffix_index.index >= self.window_start);
                if ctx.suffix_index.index == self.window_start {
                    new_active_contexts_count -= 1;
                    assert_eq!(new_active_contexts_count, order);
                }
            }
            active_contexts.keep_only(new_active_contexts_count);
        } else if node_index.is_root() {
            let root_index = node_index;
            assert_eq!(parent_node_index_opt, None);
            assert_eq!(node_direction_opt, None);
            if leaf_sibling.is_node_index() {
                let leaf_sibling_node_index = leaf_sibling.to_node_index();
                for ctx in active_contexts.items.iter_mut() {
                    if ctx.node_index == leaf_sibling_node_index {
                        ctx.node_index = root_index;
                    }
                }
                if PRINT_DEBUG {
                    println!("root node child = {:?}", leaf_direction);
                }
                assert!(leaf_sibling.is_node_index());
                let leaf_sibling_node_index = leaf_sibling.to_node_index();
                let mut leaf_sibling_node =
                    self.nodes[leaf_sibling_node_index];
                leaf_sibling_node.text_start =
                    self.nodes[root_index].text_start;
                self.nodes.update_node(root_index, leaf_sibling_node);
                self.nodes.delete_node(leaf_sibling_node_index);
                if PRINT_DEBUG { self.print(); }
            } else {
                if PRINT_DEBUG {
                    println!("root node and changing tree state to degenerate");
                }
                self.tree_state = TreeState::Degenerate;
                active_contexts.keep_only(0);
                self.nodes.update_node(root_index, Node::INVALID);
            }
        } else if node_found_in_active_contexts {
            assert!(!node_index.is_root());
            let leaf_found_in_active_contexts = active_contexts.items.iter()
                .find(|ctx| ctx.node_index.index == node_index.index &&
                    ctx.direction_from_parent == Some(leaf_direction) &&
                    ctx.in_leaf == true).is_some();
            assert!(!leaf_found_in_active_contexts,
                    "triggered situation assumed to be impossible");
            if PRINT_DEBUG {
                println!("child = {:?} (not in active contexts) \
                          of node = {:?} (in active contexts)",
                         leaf_direction, node_index);
                println!("active contexts = {}", active_contexts);
            }
            let parent_node_index = parent_node_index_opt.unwrap();
            let node_direction = node_direction_opt.unwrap();
            if leaf_sibling.is_window_index() {
                for ctx in active_contexts.items.iter_mut() {
                    if ctx.node_index == node_index {
                        if PRINT_DEBUG { print!("converted ctx = {} ", ctx); }
                        ctx.node_index = parent_node_index;
                        ctx.direction_from_parent = Some(node_direction);
                        ctx.in_leaf = true;
                        if PRINT_DEBUG { println!("to context {}", ctx); }
                    }
                }
                self.nodes[parent_node_index].children[node_direction] =
                    NodeChild::from_window_index(
                        self.nodes[node_index].text_start());
            } else {
                let leaf_sibling_node_index = leaf_sibling.to_node_index();
                for ctx in active_contexts.items.iter_mut() {
                    if ctx.node_index == node_index {
                        if PRINT_DEBUG { print!("converted ctx = {} ", ctx); }
                        ctx.node_index = leaf_sibling_node_index;
                        ctx.direction_from_parent = Some(node_direction);
                        ctx.in_leaf = false;
                        if PRINT_DEBUG { println!("to context {}", ctx); }
                    }
                }
                let mut leaf_sibling_node =
                    self.nodes[leaf_sibling_node_index];
                leaf_sibling_node.text_start =
                    self.nodes[node_index].text_start;
                self.nodes.update_node(leaf_sibling_node_index,
                                       leaf_sibling_node);
                self.nodes[parent_node_index].children[node_direction] =
                    leaf_sibling;
            }
            self.nodes.delete_node(node_index);
            if PRINT_DEBUG { self.print(); }
        } else {
            if PRINT_DEBUG {
                println!("child = {:?} of node = {:?} not in active contexts",
                         leaf_direction, node_index);
                println!("active contexts = {}", active_contexts);
            }
            assert!(!node_index.is_root());
            assert!(!node_found_in_active_contexts);
            let parent_node_index = parent_node_index_opt.unwrap();
            let node_direction = node_direction_opt.unwrap();
            self.nodes[parent_node_index].children[node_direction] =
                leaf_sibling;
            self.nodes.delete_node(node_index);
            if PRINT_DEBUG { self.print(); }
        }
        self.window[self.window_start] = 0;
        self.window_start += 1;
        self.window_size -= 1;
    }

    pub fn check_integrity(&self, max_order: usize) {
        for suffix_start in self.window_start..self.window_cursor {
            let mut node_index_opt =
                match self.tree_state {
                    TreeState::Proper => Some(self.get_root_node_index()),
                    TreeState::Degenerate => None,
                };
            while let Some(node_index) = node_index_opt {
                let node = &self.nodes[node_index];
                assert!(node.depth() <= max_order * 8 + 7);
                assert!(suffix_start <= node.text_start());
                if node.text_start() * 8 + node.depth() >=
                    self.window_cursor * 8 {
                    assert!(compare_for_equal_prefix(
                        &self.window, suffix_start, node.text_start(), 7,
                        self.window_cursor - node.text_start(),
                    ));
//                    if PRINT_DEBUG { println!("CHECK early exit"); }
                    break;
                }
                let full_byte_length = (node.depth / 8) as usize;
                let bit_index = 7 - (node.depth % 8) as usize;
                assert!(
                    compare_for_equal_prefix(
                        &self.window, suffix_start, node.text_start as usize,
                        bit_index, full_byte_length),
                    "suffix start = {}, depth bytes = {}, bit index = {}, \
                    window pos = {}, node index = {}\ninput = {:?}",
                    suffix_start, full_byte_length, bit_index,
                    self.window_cursor, node_index.index,
                    &self.window[..self.window_cursor + 1]);
                if bytes_differ_on(suffix_start + full_byte_length,
                                   node.text_start() + full_byte_length,
                                   bit_index, &self.window) {
                    break;
                }
                let bit = get_bit(self.window[suffix_start + full_byte_length],
                                  bit_index);
                let child = node.child(bit.into());
                if child.is_node_index() {
                    node_index_opt = Some(child.to_node_index());
//                    if PRINT_DEBUG { println!("CHECK descend"); }
                } else {
                    let leaf_index = child.to_window_index().index;
                    assert!(suffix_start <= leaf_index);
                    assert!(
                        compare_for_equal_prefix(
                            &self.window, suffix_start, leaf_index, 7,
                            max_order.min(self.window_cursor - leaf_index)),
                        "suffix start = {}, leaf index = {}, window pos = {}",
                        suffix_start, leaf_index, self.window_cursor);
                    node_index_opt = None;
//                    if PRINT_DEBUG { println!("CHECK leaf"); }
                }
            }
        }
        if self.tree_state == TreeState::Proper {
            let mut suffices_counters = vec![0; self.window_size];
            let mut stack = Vec::new();
            stack.push(self.get_root_node_index());
            while let Some(node_index) = stack.pop() {
                let children = self.nodes.items[node_index.index].children;
                for child in children.iter() {
                    assert!(child.is_valid());
                    if child.is_node_index() {
                        stack.push(child.to_node_index());
                    } else {
                        let window_offset =
                            child.to_window_index().index - self.window_start;
                        suffices_counters[window_offset] += 1;
                    }
                }
            }
            assert!(suffices_counters.iter().all(|&counter| counter <= 1),
                    "suffices counters = {:?}", suffices_counters);
        }
    }

    pub fn print(&self) {
        match self.tree_state {
            TreeState::Degenerate =>
                println!("Empty tree"),
            TreeState::Proper =>
                self.print_node(NodeIndex::new(self.root_index), 0),
        }
    }

    fn print_node(&self, node_index: NodeIndex, depth: usize) {
        let node = self.nodes[node_index];
        assert!(node.is_valid());
        println!("{}{} = {}", "   ".repeat(depth), node, node_index.index);
        if node.child(Direction::Left).is_node_index() {
            self.print_node(node.child(Direction::Left).to_node_index(),
                            depth + 1);
        } else {
            println!("{}{}", "   ".repeat(depth + 1),
                     node.child(Direction::Left).to_window_index().index);
        }
        if node.child(Direction::Right).is_node_index() {
            self.print_node(node.child(Direction::Right).to_node_index(),
                            depth + 1);
        } else {
            println!("{}{}", "   ".repeat(depth + 1),
                     node.child(Direction::Right).to_window_index().index);
        }
    }

    pub fn new(nodes: Nodes, max_window_size: usize, root_index: i32) -> Tree {
        assert!(max_window_size > 0);
        Tree {
            nodes,
            window: Vec::with_capacity(OVER_PROVISIONING_CONSTANT +
                max_window_size * OVER_PROVISIONING_FACTOR),
            window_start: 0,
            window_cursor: 0,
            window_size: 0,
            max_window_size,
            tree_state: TreeState::Degenerate,
            root_index,
        }
    }

    pub fn get_root_node_index(&self) -> NodeIndex {
        NodeIndex::new(self.root_index)
    }

    pub fn nodes(&self) -> &Nodes {
        &self.nodes
    }

    pub fn gather_states(&self, active_contexts: &ActiveContexts,
                         collected_states: &mut CollectedContextStates,
                         bit_index: usize) {
        collected_states.reset();
        match self.tree_state {
            TreeState::Proper => {
                assert!(self.window_cursor > self.window_start);
                for (order, context) in
                    active_contexts.items.iter().enumerate() {
                    let node: Node = self.nodes[context.node_index];
                    let last_occurrence_index = context.suffix_index.index;
                    assert!(last_occurrence_index < self.window_cursor - order);
                    let bit_history =
                        if node.depth() == order * 8 + 7 - bit_index {
                            node.history_state()
                        } else {
                            assert_ne!(context.incoming_edge_visits_count, -1);
                            let repeated_bit = get_bit(
                                self.window[order + last_occurrence_index],
                                bit_index);
                            make_bit_run_history(
                                context.incoming_edge_visits_count as usize,
                                repeated_bit)
                        };
                    if bit_history != 1 {
                        collected_states.items.push(ContextState {
                            last_occurrence_index,
                            bit_history,
                        });
                    } else {
                        assert_eq!(context.incoming_edge_visits_count, 0);
                    }
                }
            }
            TreeState::Degenerate => {
                assert_eq!(active_contexts.count(), 0);
                let count = (active_contexts.max_order() + 1)
                    .min(self.window_size - 1);
                for order in 0..count {
                    collected_states.items.push(ContextState {
                        last_occurrence_index: self.window_cursor - order - 1,
                        bit_history: make_bit_run_history(
                            self.window_size - order - 1,
                            get_bit(self.window[self.window_start], bit_index)),
                    });
                }
            }
        }
    }

    pub fn extend(&mut self, active_contexts: &mut ActiveContexts,
                  incoming_bit: bool, bit_index: usize, max_order: usize) {
        self.window[self.window_cursor] |= (incoming_bit as u8) << bit_index;
        match self.tree_state {
            TreeState::Proper => {
                let mut count = active_contexts.count();
                for order in (0..count).rev() {
                    let context = &mut active_contexts[order];
                    if context.in_leaf {
                        let child = self.nodes()[context.node_index]
                            .child(context.direction_from_parent.unwrap());
                        if !child.is_window_index() {
                            context.in_leaf = false;
                            context.node_index = child.to_node_index();
                            if PRINT_DEBUG {
                                println!("CORRECTED context = {}, order= {}",
                                         context, order);
                            }
                        }
                    }
                    let node_index = context.node_index;
                    if self.nodes()[node_index].depth()
                        == order * 8 + 7 - bit_index {
                        assert!(!context.in_leaf);
                        context.descend(self, order, bit_index);
                        if PRINT_DEBUG { self.print(); }
                    } else if bytes_differ_on(
                        context.suffix_index.index + order,
                        self.window_cursor, bit_index, &self.window,
                    ) {
                        assert!(
                            context.in_leaf || self.nodes()[node_index].depth()
                                >= order * 8,
                            "order = {}, context = {}", order, context);
                        self.split_edge(&context, order, bit_index);
                        if PRINT_DEBUG { self.print(); }
                        assert_eq!(count - 1, order);
                        count = order;
                    }
                }
                active_contexts.keep_only(count);
            }
            TreeState::Degenerate => {
                assert_eq!(active_contexts.count(), 0);
                if self.window_size >= 2 && bytes_differ_on(
                    self.window_cursor - 1, self.window_cursor, bit_index,
                    &self.window,
                ) {
                    let order = max_order.min(self.window_size - 2);
                    self.split_degenerate_root_edge(order, bit_index);
                    self.tree_state = TreeState::Proper;
                    if PRINT_DEBUG { self.print(); }
                }
            }
        }
    }

    fn nodes_mut(&mut self) -> &mut Nodes {
        &mut self.nodes
    }

    fn split_edge(&mut self, context: &Context, context_order: usize,
                  bit_index: usize) {
        let direction: Direction =
            get_bit(self.window[self.window_cursor], bit_index).into();
        let node_index = context.node_index;
        if !context.in_leaf {
            if PRINT_DEBUG {
                print!("SPLIT: internal edge, order = {}", context_order);
            }
            let mut new_node = self.nodes[node_index];
            if PRINT_DEBUG { print!(", node = {}", new_node); }
            let mut node = self.setup_split_edge(
                context, context_order, bit_index, new_node.text_start());
            new_node.text_start = context.suffix_index.index as u32;
            node.children[direction] = NodeChild::from_window_index(
                self.window_cursor - context_order);
            node.children[!direction] = self.nodes.add_node(new_node);
            if PRINT_DEBUG {
                print!(", new parent = {}, new child = {}", node, new_node);
            }
            self.nodes.update_node(node_index, node);
        } else {
            if PRINT_DEBUG {
                print!("SPLIT: leaf edge, order = {}", context_order);
            }
            let mut node = self.nodes[node_index];
            if PRINT_DEBUG { print!(", node = {}", node); }
            let mut new_node = self.setup_split_edge(
                context, context_order, bit_index,
                node.children[context.direction_from_parent.unwrap()]
                    .to_window_index().index);
            new_node.children[direction] = NodeChild::from_window_index(
                self.window_cursor - context_order);
            new_node.children[!direction] =
                NodeChild::from_window_index(context.suffix_index.index);
            node.children[context.direction_from_parent.unwrap()] =
                self.nodes.add_node(new_node);
            if PRINT_DEBUG {
                print!(", new parent = {}, new child = {}", node, new_node);
            }
            self.nodes.update_node(node_index, node);
        }
        if PRINT_DEBUG { println!(", context = {}", context); }
    }

    fn setup_split_edge(&self, context: &Context, context_order: usize,
                        bit_index: usize, text_start: usize) -> Node {
        assert_ne!(context.incoming_edge_visits_count, -1);
        let incoming_edge_visits_count =
            context.incoming_edge_visits_count as usize;
        let bit = get_bit(self.window[self.window_cursor], bit_index);
        let direction: Direction = bit.into();
        let bit_history = updated_bit_history(make_bit_run_history(
            incoming_edge_visits_count, !bit), bit);
        Node::new(text_start,
                  context_order * 8 + 7 - bit_index,
                  direction.fold(|| 1, || incoming_edge_visits_count),
                  direction.fold(|| incoming_edge_visits_count, || 1),
                  bit_history,
                  Node::INVALID.children)
    }

    fn split_degenerate_root_edge(&mut self, context_order: usize,
                                  bit_index: usize) {
        if PRINT_DEBUG {
            println!("SPLIT: Splitting degenerate root edge, order = {}",
                     context_order);
        }
        let bit = get_bit(self.window[self.window_cursor], bit_index);
        let direction: Direction = bit.into();
        let mut last_node_index_opt = None;
        for current_context_order in (0..context_order + 1).rev() {
            let distance_to_end = self.window_cursor - current_context_order;
            let branching_child = NodeChild::from_window_index(distance_to_end);
            let chained_child = last_node_index_opt.unwrap_or(
                NodeChild::from_window_index(distance_to_end - 1));
            let children = [
                direction.fold(|| branching_child, || chained_child),
                direction.fold(|| chained_child, || branching_child),
            ];
            let bit_history = updated_bit_history(make_bit_run_history(
                self.window_cursor - current_context_order, !bit), bit);
            let node = Node::new(
                distance_to_end,
                current_context_order * 8 + 7 - bit_index,
                direction.fold(|| 1, || 63.min(distance_to_end)),
                direction.fold(|| 63.min(distance_to_end), || 1),
                bit_history,
                children,
            );
            if current_context_order == 0 {
                let root_node_index = self.get_root_node_index();
                self.nodes.update_node(root_node_index, node);
                last_node_index_opt = None;
            } else {
                last_node_index_opt = Some(self.nodes.add_node(node));
            }
        }
        assert_eq!(last_node_index_opt, None);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeChild {
    index: i32
}

impl NodeChild {
    // root node can't be a child
    const INVALID: NodeChild = NodeChild { index: !0 };

    fn from_window_index(window_index: usize) -> NodeChild {
        assert!(window_index <= 0x7fff_ffff);
        NodeChild { index: window_index as i32 }
    }

    fn from_node_index(node_index: usize) -> NodeChild {
        assert!(node_index >= Nodes::NUM_ROOTS && node_index <= 0x7fff_ffff);
        NodeChild { index: !(node_index as i32) }
    }

    fn is_valid(&self) -> bool {
        self.index >= 0 || (!self.index) as usize >= Nodes::NUM_ROOTS
    }

    pub fn is_window_index(&self) -> bool {
        self.index >= 0
    }

    pub fn is_node_index(&self) -> bool {
        self.index < 0
    }

    fn to_window_index(&self) -> WindowIndex {
        WindowIndex::new(self.index)
    }

    pub fn to_node_index(&self) -> NodeIndex {
        NodeIndex::new(!self.index)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeIndex {
    index: usize
}

impl NodeIndex {
    fn new(index: i32) -> NodeIndex {
        assert!(index >= 0);
        NodeIndex { index: index as usize }
    }

    fn is_root(&self) -> bool {
        self.index < Nodes::NUM_ROOTS
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WindowIndex {
    index: usize
}

impl WindowIndex {
    fn new(index: i32) -> WindowIndex {
        assert!(index >= 0);
        WindowIndex { index: index as usize }
    }
}

#[derive(Clone, Copy)]
pub struct Node {
    children: [NodeChild; 2],
    // counter: SimpleCounter,
    text_start: u32,
    history_state: u16,
    depth: u16,
    left_count: u16,
    right_count: u16,
}

impl Node {
    const INVALID: Node = Node {
        children: [NodeChild::INVALID, NodeChild::INVALID],
        text_start: 0,
        history_state: 0,
        depth: 0,
        left_count: 0,
        right_count: 0,
    };

    fn new(text_start: usize, depth: usize,
           left_count: usize, right_count: usize, history_state: u32,
           children: [NodeChild; 2]) -> Node {
        assert!((text_start as u64) < 1u64 << 31);
        assert!((depth as u64) < 1u64 << 16);
        assert!((left_count as u64) < 1u64 << 16);
        assert!((right_count as u64) < 1u64 << 16);
        assert!((history_state as u64) < 1u64 << 16);
        Node {
            children,
            text_start: text_start as u32,
            history_state: history_state as u16,
            depth: depth as u16,
            left_count: left_count as u16,
            right_count: right_count as u16,
        }
    }

    fn is_valid(&self) -> bool {
        self.children[0] != NodeChild::INVALID &&
            self.children[1] != NodeChild::INVALID
    }

    pub fn text_start(&self) -> usize {
        self.text_start as usize
    }

    pub fn depth(&self) -> usize {
        self.depth as usize
    }

    fn left_count(&self) -> usize {
        self.left_count as usize
    }

    fn right_count(&self) -> usize {
        self.right_count as usize
    }

    fn history_state(&self) -> u32 {
        self.history_state as u32
    }

    pub fn child(&self, direction: Direction) -> NodeChild {
        self.children[direction]
    }

    fn increment_edge_counters(&mut self, direction: Direction) {
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

pub struct Nodes {
    items: Vec<Node>,
    last_deleted_node_idx_opt: Option<NodeIndex>,
    removed_nodes_count: usize,
}

impl Nodes {
    const NUM_ROOTS: usize = 1;

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

    fn add_node(&mut self, node: Node) -> NodeChild {
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
            NodeChild::from_node_index(last_deleted_node_index.index)
        } else {
            assert_eq!(self.removed_nodes_count, 0);
            assert!(self.items.capacity() > self.items.len());
            let node_child = NodeChild::from_node_index(self.items.len());
            self.items.push(node);
            node_child
        }
    }

    fn update_node(&mut self, node_index: NodeIndex, new_node: Node) {
        self.items[node_index.index] = new_node;
    }

    fn delete_node(&mut self, node_index: NodeIndex) {
        assert!(self.items[node_index.index].is_valid());
        let mut node = Node::INVALID;
        node.children[Direction::Left] = NodeChild::INVALID;
        node.children[Direction::Right] = self.last_deleted_node_idx_opt
            .map(|node_index| NodeChild::from_node_index(node_index.index))
            .unwrap_or(NodeChild::INVALID);
        self.items[node_index.index] = node;
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
        let index = node_index.index;
        let node = &self.items[index];
        assert!(index >= Nodes::NUM_ROOTS || node.is_valid());
        node
    }
}

impl ops::IndexMut<NodeIndex> for Nodes {
    fn index_mut(&mut self, node_index: NodeIndex) -> &mut Node {
        let index = node_index.index;
        let node = &mut self.items[index];
        assert!(index >= Nodes::NUM_ROOTS || node.is_valid());
        node
    }
}
