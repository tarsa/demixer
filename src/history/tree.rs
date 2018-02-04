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

use ::PRINT_DEBUG;
use history::{
    HistorySource,
    ContextState,
    CollectedContextStates,
    make_bit_run_history, updated_bit_history, get_bit, bytes_differ_on,
    compare_for_equal_prefix,
};

pub struct TreeHistorySource {
    pub tree: Tree,
    pub active_contexts: ActiveContexts,
    bit_index: usize,
}

impl HistorySource for TreeHistorySource {
    fn new(input_size: usize, max_order: usize) -> TreeHistorySource {
        let nodes = Nodes::new(input_size);
        TreeHistorySource {
            tree: Tree::new(nodes, input_size, 0),
            active_contexts: ActiveContexts::new(max_order),
            bit_index: 7,
        }
    }

    fn start_new_byte(&mut self) {
        assert_eq!(self.bit_index, 7);
        self.active_contexts.shift(&mut self.tree);
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

#[derive(Clone, Copy, Eq, PartialEq)]
enum Direction {
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
enum TreeState {
    /** Every inner node has two leaves */
    Proper,
    /** Has only invalid root node, happens when all symbols are identical */
    Degenerate,
}

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
    pub window_cursor: usize,
    tree_state: TreeState,
    root_index: i32,
}

impl Tree {
    pub fn check_integrity(&self, max_order: usize) {
        for suffix_start in 0..self.window_cursor {
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
                    assert!(
                      compare_for_equal_prefix(
                          &self.window, suffix_start, node.text_start(), 7,
                          self.window_cursor - node.text_start()
                      )
                    );
                    if PRINT_DEBUG { println!("CHECK early exit"); }
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
                    if PRINT_DEBUG { println!("CHECK descend"); }
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
                    if PRINT_DEBUG { println!("CHECK leaf"); }
                }
            }
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

    pub fn new(nodes: Nodes, window_size: usize, root_index: i32) -> Tree {
        assert!(window_size > 0);
        Tree {
            nodes,
            window: vec![0; window_size],
            window_cursor: 0,
            tree_state: TreeState::Degenerate,
            root_index,
        }
    }

    fn get_root_node_index(&self) -> NodeIndex {
        NodeIndex::new(self.root_index)
    }

    fn nodes(&self) -> &Nodes {
        &self.nodes
    }

    pub fn gather_states(&self, active_contexts: &ActiveContexts,
                         collected_states: &mut CollectedContextStates,
                         bit_index: usize) {
        collected_states.reset();
        match self.tree_state {
            TreeState::Proper => {
                assert_ne!(self.window_cursor, 0);
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
                    .min(self.window_cursor);
                for order in 0..count {
                    collected_states.items.push(ContextState {
                        last_occurrence_index: self.window_cursor - order - 1,
                        bit_history: make_bit_run_history(
                            self.window_cursor - order,
                            get_bit(self.window[0], bit_index)),
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
                        assert!(context.in_leaf ||
                            self.nodes()[node_index].depth() >= order * 8);
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
                if self.window_cursor > 0 && bytes_differ_on(
                    self.window_cursor - 1, self.window_cursor, bit_index,
                    &self.window,
                ) {
                    let order = max_order.min(self.window_cursor - 1);
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
struct NodeChild {
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

//    fn is_valid(&self) -> bool {
//        self.index >= 0 || (!self.index) as usize >= Nodes::NUM_ROOTS
//    }

    fn is_window_index(&self) -> bool {
        self.index >= 0
    }

    fn is_node_index(&self) -> bool {
        self.index < 0
    }

    fn to_window_index(&self) -> WindowIndex {
        WindowIndex::new(self.index)
    }

    fn to_node_index(&self) -> NodeIndex {
        NodeIndex::new(!self.index)
    }
}

#[derive(Clone, Copy)]
struct NodeIndex {
    index: usize
}

impl NodeIndex {
    fn new(index: i32) -> NodeIndex {
        assert!(index >= 0);
        NodeIndex { index: index as usize }
    }

//    fn is_root(&self) -> bool {
//        self.index < Nodes::NUM_ROOTS
//    }
}

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
struct Node {
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

    fn text_start(&self) -> usize {
        self.text_start as usize
    }

    fn depth(&self) -> usize {
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

    fn child(&self, direction: Direction) -> NodeChild {
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
        write!(f, "{}'{}'{:b}",
               self.text_start(), self.depth(), self.history_state())
    }
}

pub struct Nodes {
    items: Vec<Node>,
}

impl Nodes {
    const NUM_ROOTS: usize = 1;

    pub fn new(nodes_limit: usize) -> Nodes {
        assert!(nodes_limit >= Nodes::NUM_ROOTS);
        let mut items = Vec::with_capacity(nodes_limit);
        (0..Nodes::NUM_ROOTS).for_each(|_| items.push(Node::INVALID));
        Nodes { items }
    }

    fn add_node(&mut self, node: Node) -> NodeChild {
        assert!(self.items.capacity() > self.items.len());
        let node_child = NodeChild::from_node_index(self.items.len());
        self.items.push(node);
        node_child
    }

    fn update_node(&mut self, node_index: NodeIndex, new_node: Node) {
        self.items[node_index.index] = new_node;
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
