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

use PRINT_DEBUG;
use bit::Bit;
use estimators::decelerating::DeceleratingEstimator;
use lut::LookUpTables;
use super::{CollectedContextStates, ContextState, HistorySource};
use super::state::HistoryStateFactory;
use super::window::{InputWindow, WindowIndex};
use self::context::{ActiveContexts, Context};
use self::direction::Direction;
use self::node::{CostTrackers, Node};
use self::node_child::{NodeChild, NodeChildren};
use self::nodes::{NodeIndex, Nodes};

pub struct TreeHistorySource<'a> {
    pub tree: Tree<'a>,
    pub active_contexts: ActiveContexts,
    pub bit_index: i32,
}

impl<'a> TreeHistorySource<'a> {
    pub fn new_special(max_window_size: usize, max_order: usize,
                       initial_shift: usize, luts: &'a LookUpTables)
                       -> TreeHistorySource<'a> {
        assert!(max_window_size > 0);
        assert!(initial_shift < max_window_size);
        let nodes = Nodes::new(Nodes::NUM_ROOTS.max(max_window_size - 1));
        TreeHistorySource {
            tree: Tree::new(nodes, max_window_size, initial_shift, 0, luts),
            active_contexts: ActiveContexts::new(max_order),
            bit_index: -1,
        }
    }

    pub fn check_integrity_on_next_byte(&self) {
        assert_eq!(self.bit_index, 7);
        self.active_contexts.check_integrity_on_next_byte(&self.tree);
        let max_order = self.active_contexts.max_order();
        self.tree.check_integrity_on_next_byte(max_order);
    }
}

impl<'a> HistorySource<'a> for TreeHistorySource<'a> {
    fn new(max_window_size: usize, max_order: usize, luts: &'a LookUpTables)
           -> TreeHistorySource<'a> {
        TreeHistorySource::new_special(max_window_size, max_order, 0, luts)
    }

    fn start_new_byte(&mut self) {
        assert_eq!(self.bit_index, -1);
        self.bit_index = 7;
        self.tree.start_new_byte(&mut self.active_contexts);
        self.active_contexts.shift(&mut self.tree);
    }

    fn gather_history_states(&self,
                             collected_states: &mut CollectedContextStates) {
        assert!(self.bit_index >= 0);
        self.tree.gather_states(&self.active_contexts, collected_states,
                                self.bit_index as usize);
    }

    fn process_input_bit(&mut self, input_bit: Bit,
                         new_cost_trackers: &[CostTrackers]) {
        assert!(self.bit_index >= 0);
        let max_order = self.active_contexts.max_order();
        self.tree.extend(&mut self.active_contexts, new_cost_trackers,
                         input_bit, self.bit_index as usize, max_order);
        self.bit_index -= 1;
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TreeState {
    /** Every inner node has two leaves */
    Proper,
    /** Has only invalid root node, happens when all symbols are identical */
    Degenerate,
}

pub struct Tree<'a> {
    luts: &'a LookUpTables,
    nodes: Nodes,
    pub window: InputWindow,
    tree_state: TreeState,
    root_index: usize,
}

impl<'a> Tree<'a> {
    pub fn tree_state(&self) -> TreeState {
        self.tree_state
    }

    fn start_new_byte(&mut self, active_contexts: &mut ActiveContexts) {
        if self.window.size() == self.window.max_size() {
            self.remove_leftmost_suffix(active_contexts);
            let next_cursor = self.window.index_increment(self.window.cursor());
            assert_eq!(self.window[next_cursor], 0);
        }
        assert!(self.window.size() < self.window.max_size());
        self.window.advance_cursor();
    }

    pub fn remove_leftmost_suffix(&mut self,
                                  active_contexts: &mut ActiveContexts) {
        if self.tree_state == TreeState::Degenerate {
            self.window.advance_start();
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
                    self.window.index_add(self.window.start(), depth / 8);
                self.window.get_bit(byte_index, 7 - (depth % 8)).into()
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
            .find(|ctx| ctx.node_index == node_index).is_some();
        if PRINT_DEBUG { print!("DELETING: "); }
        if leaf_window_index != self.window.start() {
            if PRINT_DEBUG {
                println!("skipped because prefix was repeated");
                println!("window start = {}, active contexts = {}",
                         self.window.start(), active_contexts);
            }
            let mut new_active_contexts_count = active_contexts.count();
            for (order, ctx) in active_contexts.items.iter().enumerate().rev() {
                if ctx.suffix_index == self.window.start() {
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
                .find(|ctx| ctx.node_index == node_index &&
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
                    self.nodes[node_index].text_start().into();
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
        self.window.advance_start();
    }

    pub fn check_integrity_on_next_byte(&self, max_order: usize) {
        assert!(self.window.size() == self.window.max_size() ||
            (self.window.index_diff(self.window.cursor(), self.window.start()) +
                1 == self.window.size()));
        // check that all suffices are present in tree
        self.window.for_each_suffix(|suffix_start| {
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
                if node.depth() / 8 >= self.window.index_diff(
                    self.window.cursor(), node.text_start()) {
                    assert!(self.window.compare_for_equal_prefix(
                        suffix_start, node.text_start(), 7,
                        self.window.index_diff(
                            self.window.cursor(), node.text_start())));
//                    if PRINT_DEBUG { println!("CHECK early exit"); }
                    break;
                }
                let full_byte_length = node.depth() / 8;
                let bit_index = 7 - (node.depth() % 8);
                assert!(
                    self.window.compare_for_equal_prefix(
                        suffix_start, node.text_start(),
                        bit_index, full_byte_length),
                    "suffix start = {}, depth bytes = {}, bit index = {}, \
                    window pos = {}, node index = {}",
                    suffix_start, full_byte_length, bit_index,
                    self.window.cursor(), node_index);
                if self.window.bytes_differ_on(
                    self.window.index_add(suffix_start, full_byte_length),
                    self.window.index_add(node.text_start(), full_byte_length),
                    bit_index) {
                    break;
                }
                let bit = {
                    let byte_index =
                        self.window.index_add(suffix_start, full_byte_length);
                    self.window.get_bit(byte_index, bit_index)
                };
                let child = node.child(bit.into());
                if child.is_node_index() {
                    node_index_opt = Some(child.to_node_index());
//                    if PRINT_DEBUG { println!("CHECK descend"); }
                } else {
                    let leaf_index = child.to_window_index();
                    assert!(self.window.index_is_smaller_or_equal(
                        suffix_start, leaf_index));
                    assert!(
                        self.window.compare_for_equal_prefix(
                            suffix_start, leaf_index, 7,
                            max_order.min(self.window.index_diff(
                                self.window.cursor(), leaf_index))),
                        "suffix start = {}, leaf index = {}, window pos = {}",
                        suffix_start, leaf_index, self.window.cursor());
                    node_index_opt = None;
//                    if PRINT_DEBUG { println!("CHECK leaf"); }
                }
            }
        });
        // check that all leaves correspond to different suffices
        if self.tree_state == TreeState::Proper {
            let mut suffices_counters = vec![0u8; self.window.size()];
            let mut stack = Vec::new();
            stack.push(self.get_root_node_index());
            while let Some(node_index) = stack.pop() {
                let children = &self.nodes[node_index].children;
                for child in children.items().iter() {
                    assert!(child.is_valid());
                    if child.is_node_index() {
                        stack.push(child.to_node_index());
                    } else {
                        let window_offset = self.window.index_diff(
                            child.to_window_index(), self.window.start());
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
        println!("{}{} = {}", "   ".repeat(depth), node, node_index);
        if node.child(Direction::Left).is_node_index() {
            self.print_node(node.child(Direction::Left).to_node_index(),
                            depth + 1);
        } else {
            println!("{}{}", "   ".repeat(depth + 1),
                     node.child(Direction::Left).to_window_index());
        }
        if node.child(Direction::Right).is_node_index() {
            self.print_node(node.child(Direction::Right).to_node_index(),
                            depth + 1);
        } else {
            println!("{}{}", "   ".repeat(depth + 1),
                     node.child(Direction::Right).to_window_index());
        }
    }

    pub fn new(nodes: Nodes, max_window_size: usize, initial_shift: usize,
               root_index: usize, luts: &LookUpTables) -> Tree {
        Tree {
            luts,
            nodes,
            window: InputWindow::new(max_window_size, initial_shift),
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
        assert_eq!(collected_states.items().len(), 0);
        match self.tree_state {
            TreeState::Proper => {
                assert_ne!(self.window.cursor(), self.window.start());
                for (order, context) in
                    active_contexts.items.iter().enumerate() {
                    let node = &self.nodes[context.node_index];
                    let last_occurrence_index = context.suffix_index;
                    let current_occurrence_index = self.window.index_subtract(
                        self.window.cursor(), order);
                    assert!(self.window.index_is_smaller(
                        last_occurrence_index, current_occurrence_index));
                    let last_occurrence_distance = self.window.index_diff(
                        current_occurrence_index, last_occurrence_index);
                    if node.depth() == order * 8 + 7 - bit_index {
                        collected_states.items.push(ContextState::ForNode {
                            last_occurrence_distance,
                            probability_estimator: node.probability_estimator(),
                            bit_history: node.history_state(),
                            cost_trackers: node.cost_trackers(),
                        });
                    } else {
                        assert_ne!(context.incoming_edge_visits_count, -1);
                        if context.incoming_edge_visits_count > 0 {
                            let repeated_bit = self.window.get_bit(
                                self.window.index_add(
                                    last_occurrence_index, order), bit_index);
                            collected_states.items.push(ContextState::ForEdge {
                                last_occurrence_distance,
                                repeated_bit,
                                occurrence_count:
                                context.incoming_edge_visits_count as u16,
                            });
                        }
                    }
                }
            }
            TreeState::Degenerate => {
                assert_eq!(active_contexts.count(), 0);
                let count = (active_contexts.max_order() + 1)
                    .min(self.window.size() - 1);
                for order in 0..count {
                    collected_states.items.push(ContextState::ForEdge {
                        last_occurrence_distance: 1,
                        occurrence_count:
                        (ContextState::MAX_OCCURRENCE_COUNT as usize)
                            .min(self.window.size() - order - 1) as u16,
                        repeated_bit: self.window.get_bit(
                            self.window.start(), bit_index),
                    });
                }
            }
        }
    }

    pub fn extend(&mut self, active_contexts: &mut ActiveContexts,
                  new_cost_trackers: &[CostTrackers],
                  incoming_bit: Bit, bit_index: usize, max_order: usize) {
        self.window.set_bit_at_cursor(incoming_bit, bit_index);
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
                        assert!(order < new_cost_trackers.len());
                        context.descend(self, order, bit_index,
                                        new_cost_trackers[order].clone(),
                                        self.luts.d_estimator_rates());
                        if PRINT_DEBUG { self.print(); }
                    } else {
                        assert!(order >= new_cost_trackers.len());
                        if self.window.bytes_differ_on(
                            self.window.index_add(context.suffix_index, order),
                            self.window.cursor(), bit_index,
                        ) {
                            let node_depth = self.nodes()[node_index].depth();
                            assert!(context.in_leaf || node_depth >= order * 8,
                                    "order = {}, context = {}", order, context);
                            self.split_edge(&context, order, bit_index,
                                            self.luts);
                            if PRINT_DEBUG { self.print(); }
                            assert_eq!(count - 1, order);
                            count = order;
                        }
                    }
                }
                active_contexts.keep_only(count);
            }
            TreeState::Degenerate => {
                assert_eq!(active_contexts.count(), 0);
                if self.window.size() >= 2 && self.window.bytes_differ_on(
                    self.window.index_decrement(self.window.cursor()),
                    self.window.cursor(), bit_index,
                ) {
                    let order = max_order.min(self.window.size() - 2);
                    self.split_degenerate_root_edge(order, bit_index,
                                                    self.luts);
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
                  bit_index: usize, luts: &LookUpTables) {
        let direction: Direction =
            self.window.get_bit(self.window.cursor(), bit_index).into();
        let node_index = context.node_index;
        if !context.in_leaf {
            if PRINT_DEBUG {
                print!("SPLIT: internal edge, order = {}", context_order);
            }
            let mut new_node = self.nodes[node_index].clone();
            if PRINT_DEBUG { print!(", node = {}", new_node); }
            let mut node = self.setup_split_edge(
                context, context_order, bit_index, new_node.text_start(), luts);
            new_node.text_start = context.suffix_index.raw() as u32;
            node.children[direction] = self.window.index_subtract(
                self.window.cursor(), context_order).into();
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
                    .to_window_index(), luts);
            new_node.children[direction] = self.window.index_subtract(
                self.window.cursor(), context_order).into();
            new_node.children[!direction] = context.suffix_index.into();
            if PRINT_DEBUG { print!(", new child = {}", new_node) }
            node.children[context.direction_from_parent.unwrap()] =
                self.nodes.add_node(new_node);
            if PRINT_DEBUG { print!(", new parent = {}", node); }
            self.nodes.update_node(node_index, node);
        }
        if PRINT_DEBUG { println!(", context = {}", context); }
    }

    fn setup_split_edge(&self, context: &Context, context_order: usize,
                        bit_index: usize, text_start: WindowIndex,
                        luts: &LookUpTables) -> Node {
        assert_ne!(context.incoming_edge_visits_count, -1);
        let incoming_edge_visits_count =
            context.incoming_edge_visits_count as u16;
        let bit = self.window.get_bit(self.window.cursor(), bit_index);
        let direction: Direction = bit.into();
        let probability_estimator = luts.d_estimator_cache().for_new_node(
            bit,
            DeceleratingEstimator::MAX_COUNT.min(incoming_edge_visits_count));
        let cost_tracker = luts.cost_trackers_lut()
            .for_new_node(incoming_edge_visits_count);
        let cost_trackers = CostTrackers::new(cost_tracker, cost_tracker);
        let bit_history = luts.history_state_factory()
            .for_new_node(bit, incoming_edge_visits_count);
        Node::new(text_start,
                  probability_estimator,
                  context_order * 8 + 7 - bit_index,
                  cost_trackers,
                  direction.fold(|| 1, || incoming_edge_visits_count),
                  direction.fold(|| incoming_edge_visits_count, || 1),
                  bit_history,
                  NodeChildren::INVALID)
    }

    fn split_degenerate_root_edge(&mut self, context_order: usize,
                                  bit_index: usize, luts: &LookUpTables) {
        if PRINT_DEBUG {
            println!("SPLIT: Splitting degenerate root edge, order = {}",
                     context_order);
        }
        let bit = self.window.get_bit(self.window.cursor(), bit_index);
        let direction: Direction = bit.into();
        let mut last_node_index_opt = None;
        for current_context_order in (0..context_order + 1).rev() {
            let suffix_start = self.window.index_subtract(
                self.window.cursor(), current_context_order);
            let branching_child: NodeChild = suffix_start.into();
            let chained_child = last_node_index_opt.unwrap_or(
                self.window.index_decrement(suffix_start).into());
            let children = NodeChildren::new([
                direction.fold(|| branching_child, || chained_child),
                direction.fold(|| chained_child, || branching_child),
            ]);
            let repeated_edge_count =
                (ContextState::MAX_OCCURRENCE_COUNT as usize).min(
                    self.window.index_diff(suffix_start, self.window.start())
                ) as u16;
            let probability_estimator = luts.d_estimator_cache()
                .for_new_node(bit, repeated_edge_count);
            let bit_history = luts.history_state_factory()
                .for_new_node(bit, repeated_edge_count);
            let cost_tracker = luts.cost_trackers_lut()
                .for_new_node(repeated_edge_count);
            let cost_trackers = CostTrackers::new(cost_tracker, cost_tracker);
            let node = Node::new(
                suffix_start,
                probability_estimator,
                current_context_order * 8 + 7 - bit_index,
                cost_trackers,
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
