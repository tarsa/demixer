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
pub mod context;
pub mod direction;
pub mod node;
pub mod node_child;
pub mod nodes;
pub mod window;

use ::PRINT_DEBUG;
use ::history::{
    CollectedContextStates, ContextState, HistorySource,
    get_bit, make_bit_run_history, updated_bit_history,
};
use self::context::{ActiveContexts, Context};
use self::direction::Direction;
use self::node::Node;
use self::node_child::{NodeChild, NodeIndex};
use self::nodes::Nodes;
use self::window::InputWindow;

pub struct TreeHistorySource {
    pub tree: Tree,
    pub active_contexts: ActiveContexts,
    bit_index: usize,
}

impl TreeHistorySource {
    pub fn new_special(max_window_size: usize, max_order: usize,
                       initial_shift: usize) -> TreeHistorySource {
        assert!(max_window_size > 0);
        assert!(initial_shift < max_window_size);
        let nodes = Nodes::new(Nodes::NUM_ROOTS.max(max_window_size - 1));
        TreeHistorySource {
            tree: Tree::new(nodes, max_window_size, initial_shift, 0),
            active_contexts: ActiveContexts::new(max_order),
            bit_index: 7,
        }
    }

    pub fn check_integrity_before_next_byte(&self) {
        assert_eq!(self.bit_index, 7);
        self.active_contexts.check_integrity_before_next_byte(&self.tree);
        let max_order = self.active_contexts.max_order();
        self.tree.check_integrity_before_next_byte(max_order);
    }
}

impl HistorySource for TreeHistorySource {
    fn new(max_window_size: usize, max_order: usize) -> TreeHistorySource {
        TreeHistorySource::new_special(max_window_size, max_order, 0)
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
            self.tree.window.cursor += 1;
            if self.tree.window.cursor == self.tree.window.max_size {
                self.tree.window.cursor = 0;
            }
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

pub struct Tree {
    nodes: Nodes,
    pub window: InputWindow,
    pub tree_state: TreeState,
    root_index: i32,
}

impl Tree {
    fn start_new_byte(&mut self, active_contexts: &mut ActiveContexts) {
        if self.window.size == self.window.max_size {
            assert_eq!(self.window.start, self.window.cursor);
            assert_eq!(self.window.max_size, self.window.buffer.len());
            self.remove_leftmost_suffix(active_contexts);
            assert_eq!(self.window.buffer[self.window.cursor], 0);
        } else {
            assert!(self.window.size < self.window.max_size);
            if self.window.buffer.len() < self.window.buffer.capacity() {
                self.window.buffer.push(0);
            } else {
                self.window.buffer[self.window.cursor] = 0;
            }
        }
        self.window.size += 1;
    }

    pub fn remove_leftmost_suffix(&mut self,
                                  active_contexts: &mut ActiveContexts) {
        if self.tree_state == TreeState::Degenerate {
            self.window.buffer[self.window.start] = 0;
            self.window.start = self.window.index_increment(self.window.start);
            self.window.size -= 1;
            return;
        }
        let mut parent_node_index_opt = None;
        let mut node_direction_opt = None;
        let mut node_index = self.get_root_node_index();
        let mut leaf_direction_opt = None;
        let mut leaf_sibling_opt = None;
        while leaf_direction_opt == None {
            let depth = self.nodes[node_index].depth();
            let direction: Direction = {
                let byte_index =
                    self.window.index_add(self.window.start, depth / 8);
                get_bit(self.window.buffer[byte_index], 7 - (depth % 8)).into()
            };
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
        if leaf_window_index.index != self.window.start {
            if PRINT_DEBUG {
                println!("skipped because prefix was repeated");
                println!("window start = {}, active contexts = {}",
                         self.window.start, active_contexts);
            }
            let mut new_active_contexts_count = active_contexts.count();
            for (order, ctx) in active_contexts.items.iter().enumerate().rev() {
                if ctx.suffix_index.index == self.window.start {
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
                    self.nodes[leaf_sibling_node_index].clone();
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
                    self.nodes[leaf_sibling_node_index].clone();
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
        self.window.buffer[self.window.start] = 0;
        self.window.start = self.window.index_increment(self.window.start);
        self.window.size -= 1;
    }

    pub fn check_integrity_before_next_byte(&self, max_order: usize) {
        assert!(self.window.size == self.window.max_size ||
            (self.window.index_subtract(self.window.cursor, self.window.start)
                == self.window.size));
        // check that all suffices are present in tree
        let suffices_range =
            if self.window.size == 0 || self.window.start < self.window.cursor {
                (self.window.start..self.window.cursor).chain(0..0)
            } else {
                (self.window.start..self.window.max_size)
                    .chain(0..self.window.cursor)
            };
        for suffix_start in suffices_range {
            let mut node_index_opt =
                match self.tree_state {
                    TreeState::Proper => Some(self.get_root_node_index()),
                    TreeState::Degenerate => None,
                };
            while let Some(node_index) = node_index_opt {
                let node = &self.nodes[node_index];
                assert!(node.depth() <= max_order * 8 + 7);
                assert!(self.window.index_is_smaller_or_equal(
                    suffix_start, node.text_start()));
                if node.depth() / 8 >= self.window.index_subtract(
                    self.window.cursor, node.text_start()) {
                    assert!(self.window.compare_for_equal_prefix(
                        suffix_start, node.text_start(), 7,
                        self.window.index_subtract(
                            self.window.cursor, node.text_start())));
//                    if PRINT_DEBUG { println!("CHECK early exit"); }
                    break;
                }
                let full_byte_length = (node.depth / 8) as usize;
                let bit_index = 7 - (node.depth % 8) as usize;
                assert!(
                    self.window.compare_for_equal_prefix(
                        suffix_start, node.text_start as usize,
                        bit_index, full_byte_length),
                    "suffix start = {}, depth bytes = {}, bit index = {}, \
                    window pos = {}, node index = {}",
                    suffix_start, full_byte_length, bit_index,
                    self.window.cursor, node_index.index);
                if self.window.bytes_differ_on(
                    self.window.index_add(suffix_start, full_byte_length),
                    self.window.index_add(node.text_start(), full_byte_length),
                    bit_index) {
                    break;
                }
                let bit = {
                    let byte_index =
                        self.window.index_add(suffix_start, full_byte_length);
                    get_bit(self.window.buffer[byte_index], bit_index)
                };
                let child = node.child(bit.into());
                if child.is_node_index() {
                    node_index_opt = Some(child.to_node_index());
//                    if PRINT_DEBUG { println!("CHECK descend"); }
                } else {
                    let leaf_index = child.to_window_index().index;
                    assert!(self.window.index_is_smaller_or_equal(
                        suffix_start, leaf_index));
                    assert!(
                        self.window.compare_for_equal_prefix(
                            suffix_start, leaf_index, 7,
                            max_order.min(self.window.index_subtract(
                                self.window.cursor, leaf_index))),
                        "suffix start = {}, leaf index = {}, window pos = {}",
                        suffix_start, leaf_index, self.window.cursor);
                    node_index_opt = None;
//                    if PRINT_DEBUG { println!("CHECK leaf"); }
                }
            }
        }
        // check that all leaves correspond to different suffices
        if self.tree_state == TreeState::Proper {
            let mut suffices_counters = vec![0u8; self.window.size];
            let mut stack = Vec::new();
            stack.push(self.get_root_node_index());
            while let Some(node_index) = stack.pop() {
                let children = self.nodes.items[node_index.index].children;
                for child in children.iter() {
                    assert!(child.is_valid());
                    if child.is_node_index() {
                        stack.push(child.to_node_index());
                    } else {
                        let window_offset = self.window.index_subtract(
                            child.to_window_index().index, self.window.start);
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
        let node = &self.nodes[node_index];
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

    pub fn new(nodes: Nodes, max_window_size: usize, initial_shift: usize,
               root_index: i32) -> Tree {
        assert!(max_window_size > 0);
        let mut buffer = Vec::with_capacity(max_window_size);
        buffer.resize(initial_shift, 0);
        assert_eq!(buffer.capacity(), max_window_size);
        assert_eq!(buffer.len(), initial_shift);
        Tree {
            nodes,
            window: InputWindow {
                buffer,
                start: initial_shift,
                cursor: initial_shift,
                size: 0,
                max_size: max_window_size,
            },
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
                assert_ne!(self.window.cursor, self.window.start);
                for (order, context) in
                    active_contexts.items.iter().enumerate() {
                    let node = &self.nodes[context.node_index];
                    let last_occurrence_index = context.suffix_index.index;
                    assert!(self.window.index_is_smaller(
                        last_occurrence_index, self.window.index_subtract(
                            self.window.cursor, order)));
                    let bit_history =
                        if node.depth() == order * 8 + 7 - bit_index {
                            node.history_state()
                        } else {
                            assert_ne!(context.incoming_edge_visits_count, -1);
                            let repeated_bit = get_bit(
                                self.window.buffer[self.window.index_add(
                                    last_occurrence_index, order)], bit_index);
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
                    .min(self.window.size - 1);
                for order in 0..count {
                    collected_states.items.push(ContextState {
                        last_occurrence_index: self.window.index_subtract(
                            self.window.cursor, order + 1),
                        bit_history: make_bit_run_history(
                            self.window.size - order - 1,
                            get_bit(self.window.buffer[self.window.start],
                                    bit_index)),
                    });
                }
            }
        }
    }

    pub fn extend(&mut self, active_contexts: &mut ActiveContexts,
                  incoming_bit: bool, bit_index: usize, max_order: usize) {
        self.window.buffer[self.window.cursor] |=
            (incoming_bit as u8) << bit_index;
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
                    } else if self.window.bytes_differ_on(
                        self.window.index_add(
                            context.suffix_index.index, order),
                        self.window.cursor, bit_index,
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
                if self.window.size >= 2 && self.window.bytes_differ_on(
                    self.window.index_decrement(self.window.cursor),
                    self.window.cursor, bit_index,
                ) {
                    let order = max_order.min(self.window.size - 2);
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
            get_bit(self.window.buffer[self.window.cursor], bit_index).into();
        let node_index = context.node_index;
        if !context.in_leaf {
            if PRINT_DEBUG {
                print!("SPLIT: internal edge, order = {}", context_order);
            }
            let mut new_node = self.nodes[node_index].clone();
            if PRINT_DEBUG { print!(", node = {}", new_node); }
            let mut node = self.setup_split_edge(
                context, context_order, bit_index, new_node.text_start());
            new_node.text_start = context.suffix_index.index as u32;
            node.children[direction] = NodeChild::from_window_index(
                self.window.index_subtract(self.window.cursor, context_order));
            if PRINT_DEBUG { print!(", new child = {}", new_node) }
            node.children[!direction] = self.nodes.add_node(new_node);
            if PRINT_DEBUG { print!(", new parent = {}", node); }
            self.nodes.update_node(node_index, node);
        } else {
            if PRINT_DEBUG {
                print!("SPLIT: leaf edge, order = {}", context_order);
            }
            let mut node = self.nodes[node_index].clone();
            if PRINT_DEBUG { print!(", node = {}", node); }
            let mut new_node = self.setup_split_edge(
                context, context_order, bit_index,
                node.children[context.direction_from_parent.unwrap()]
                    .to_window_index().index);
            new_node.children[direction] = NodeChild::from_window_index(
                self.window.index_subtract(self.window.cursor, context_order));
            new_node.children[!direction] =
                NodeChild::from_window_index(context.suffix_index.index);
            if PRINT_DEBUG { print!(", new child = {}", new_node) }
            node.children[context.direction_from_parent.unwrap()] =
                self.nodes.add_node(new_node);
            if PRINT_DEBUG { print!(", new parent = {}", node); }
            self.nodes.update_node(node_index, node);
        }
        if PRINT_DEBUG { println!(", context = {}", context); }
    }

    fn setup_split_edge(&self, context: &Context, context_order: usize,
                        bit_index: usize, text_start: usize) -> Node {
        assert_ne!(context.incoming_edge_visits_count, -1);
        let incoming_edge_visits_count =
            context.incoming_edge_visits_count as usize;
        let bit = get_bit(self.window.buffer[self.window.cursor], bit_index);
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
        let bit = get_bit(self.window.buffer[self.window.cursor], bit_index);
        let direction: Direction = bit.into();
        let mut last_node_index_opt = None;
        for current_context_order in (0..context_order + 1).rev() {
            let suffix_start = self.window.index_subtract(
                self.window.cursor, current_context_order);
            let branching_child = NodeChild::from_window_index(suffix_start);
            let chained_child = last_node_index_opt.unwrap_or(
                NodeChild::from_window_index(
                    self.window.index_decrement(suffix_start)));
            let children = [
                direction.fold(|| branching_child, || chained_child),
                direction.fold(|| chained_child, || branching_child),
            ];
            let bit_history = updated_bit_history(make_bit_run_history(
                self.window.size - current_context_order - 1, !bit), bit);
            let repeated_edge_count = 63.min(
                self.window.index_subtract(suffix_start, self.window.start));
            let node = Node::new(
                suffix_start,
                current_context_order * 8 + 7 - bit_index,
                direction.fold(|| 1, || repeated_edge_count),
                direction.fold(|| repeated_edge_count, || 1),
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
