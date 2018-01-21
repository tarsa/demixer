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

fn make_bit_run_history(uncapped_length: usize, repeated_bit: bool) -> u32 {
    let length = 10.min(uncapped_length);
    let bit = repeated_bit as u32;
    (1 << length) | (((1 << length) - 1) * bit)
}

fn updated_bit_history(bit_history: u32, next_bit: bool) -> u32 {
    ((bit_history << 1) & 2047) | (next_bit as u32) | (bit_history & 1024)
}

fn get_bit(byte: u8, bit_index: usize) -> bool {
    ((byte >> bit_index) & 1) == 1
}

fn bytes_differ_on(first_byte_index: usize, second_byte_index: usize,
                   bit_index: usize, input_block: &[u8]) -> bool {
    get_bit(input_block[first_byte_index] ^ input_block[second_byte_index],
            bit_index)
}

pub struct CollectedBitHistories {
    pub items: Vec<u32>, // TODO: wrap u32 in BitHistory
}

impl CollectedBitHistories {
    pub fn new(max_order: usize) -> CollectedBitHistories {
        CollectedBitHistories {
            items: Vec::with_capacity(max_order + 1)
        }
    }

    fn reset(&mut self) {
        self.items.clear();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TreeState {
    /** Every inner node has two leaves */
    Proper,
    /** Has only invalid root node, happens when all symbols are identical */
    Degenerate,
}

pub struct Context {
    node_index: NodeIndex,
    incoming_edge_visits_count: i32,
    in_leaf: bool,
    direction_from_parent: Option<Direction>,
}

impl Context {
    fn descend(&mut self, tree: &mut Tree, bit_index: usize) {
        assert!(!self.in_leaf);
        let direction: Direction =
            get_bit(tree.window[tree.window_cursor], bit_index).into();
        self.direction_from_parent = Some(direction);
        let node_index = self.node_index;
        let node = &mut tree.nodes_mut()[node_index];
        self.incoming_edge_visits_count =
            if direction == Direction::Left {
                node.left_count()
            } else {
                node.right_count()
            } as i32;
        node.increment_edge_counters(direction);
        if node.child(direction).is_window_index() {
            self.in_leaf = true;
        } else {
            self.node_index = node.child(direction).to_node_index();
        }
    }

    fn active_suffix_start(&self, nodes: &Nodes) -> usize {
        let node_index = self.node_index;
        let node = &nodes[node_index];
        if self.in_leaf {
            node.child(self.direction_from_parent.unwrap())
                .to_window_index().index
        } else {
            node.text_start()
        }
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

    pub fn shift(&mut self, tree: &Tree) {
        if tree.tree_state == TreeState::Degenerate {
            assert_eq!(self.count(), 0);
            return;
        }
        if self.max_order() + 1 == self.items.len() {
            self.items.pop().unwrap();
        }
        let root_index = tree.get_root_node_index();
        let root = &tree.nodes[root_index];
        let incoming_edge_visits_count =
            63.min(root.left_count() + root.right_count()) as i32;
        self.items.insert(0, Context {
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

pub struct Tree {
    nodes: Nodes,
    window: Vec<u8>,
    pub window_cursor: usize,
    tree_state: TreeState,
    root_index: i32,
}

impl Tree {
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
                         collected_states: &mut CollectedBitHistories,
                         bit_index: usize) {
        collected_states.reset();
        match self.tree_state {
            TreeState::Proper => {
                assert_ne!(self.window_cursor, 0);
                for (order, context) in
                    active_contexts.items.iter().enumerate() {
                    let node_index = context.node_index;
                    let node = self.nodes[node_index];
                    let bit_history =
                        if node.depth() == order * 8 + 7 - bit_index {
                            node.history_state()
                        } else {
                            assert_ne!(context.incoming_edge_visits_count, -1);
                            if context.in_leaf {
                                make_bit_run_history(
                                    context.incoming_edge_visits_count as usize,
                                    get_bit(
                                        self.window[order + node.child(context
                                            .direction_from_parent.unwrap())
                                            .to_window_index().index],
                                        bit_index,
                                    ),
                                )
                            } else {
                                make_bit_run_history(
                                    context.incoming_edge_visits_count as usize,
                                    get_bit(
                                        self.window[order + node.text_start()],
                                        bit_index,
                                    ),
                                )
                            }
                        };
                    collected_states.items.push(bit_history);
                }
            }
            TreeState::Degenerate => {
                assert_eq!(active_contexts.count(), 0);
                let count = (active_contexts.max_order() + 1)
                    .min(self.window_cursor);
                for order in 0..count {
                    collected_states.items.push(
                        make_bit_run_history(
                            count - order,
                            get_bit(self.window[0], bit_index))
                    );
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
                        }
                    }
                    let node_index = context.node_index;
                    if self.nodes()[node_index].depth()
                        == order * 8 + 7 - bit_index {
                        assert!(!context.in_leaf);
                        context.descend(self, bit_index);
                    } else if bytes_differ_on(
                        context.active_suffix_start(self.nodes()) + order,
                        self.window_cursor, bit_index, &self.window) {
                        assert!(context.in_leaf ||
                            self.nodes()[node_index].depth() >= order * 8);
                        self.split_edge(&context, order, bit_index);
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
                    &self.window) {
                    let order = max_order.min(self.window_cursor);
                    self.split_degenerate_root_edge(order, bit_index);
                    self.tree_state = TreeState::Proper;
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
            let new_node = self.nodes[node_index];
            let mut node = self.setup_split_edge(context, context_order,
                                                 bit_index);
            node.children[direction] =
                NodeChild::from_window_index(node.text_start());
            node.children[!direction] = self.nodes.add_node(new_node);
            self.nodes.update_node(node_index, node);
        } else {
            let mut new_node = self.setup_split_edge(context, context_order,
                                                     bit_index);
            let mut node = self.nodes[node_index];
            new_node.children[direction] =
                NodeChild::from_window_index(new_node.text_start());
            new_node.children[!direction] =
                node.children[context.direction_from_parent.unwrap()];
            node.children[context.direction_from_parent.unwrap()] =
                self.nodes.add_node(new_node);
            self.nodes.update_node(node_index, node);
        }
    }

    fn setup_split_edge(&self, context: &Context, context_order: usize,
                        bit_index: usize) -> Node {
        assert_ne!(context.incoming_edge_visits_count, -1);
        let incoming_edge_visits_count =
            context.incoming_edge_visits_count as usize;
        let bit = get_bit(self.window[self.window_cursor], bit_index);
        let direction: Direction = bit.into();
        let bit_history = updated_bit_history(make_bit_run_history(
            incoming_edge_visits_count, !bit), bit);
        Node::new(self.window_cursor - context_order,
                  context_order * 8 + 7 - bit_index,
                  direction.fold(|| 1, || incoming_edge_visits_count),
                  direction.fold(|| incoming_edge_visits_count, || 1),
                  bit_history,
                  Node::INVALID.children)
    }

    fn split_degenerate_root_edge(&mut self, context_order: usize,
                                  bit_index: usize) {
        let bit = get_bit(self.window[self.window_cursor], bit_index);
        let direction: Direction = bit.into();
        let mut last_node_index = None;
        for current_context_order in (0..context_order + 1).rev() {
            let distance_to_end = self.window_cursor - current_context_order;
            let branching_child = NodeChild::from_window_index(distance_to_end);
            let chained_child = last_node_index.unwrap_or(
                NodeChild::from_window_index(0));
            let children = [
                direction.fold(|| branching_child, || chained_child),
                direction.fold(|| chained_child, || branching_child),
            ];
            let bit_history = updated_bit_history(make_bit_run_history(
                self.window_cursor - current_context_order, !bit), bit);
            let node = Node::new(
                self.window_cursor - current_context_order,
                current_context_order * 8 + 7 - bit_index,
                direction.fold(|| 1, || 63.min(distance_to_end)),
                direction.fold(|| 63.min(distance_to_end), || 1),
                bit_history,
                children,
            );
            if current_context_order == 0 {
                let root_node_index = self.get_root_node_index();
                self.nodes.update_node(root_node_index, node);
            } else {
                last_node_index = Some(self.nodes.add_node(node));
            }
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
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

    fn is_valid(&self) -> bool {
        self.index >= 0 || (!self.index) as usize >= Nodes::NUM_ROOTS
    }

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

    fn is_root(&self) -> bool {
        self.index < Nodes::NUM_ROOTS
    }
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
    packed: u64,
    children: [NodeChild; 2],
    // counter: SimpleCounter,
}

impl Node {
    const TEXT_START_BITS: i32 = 31;
    const DEPTH_BITS: i32 = 9;
    const LEFT_COUNT_BITS: i32 = 6;
    const RIGHT_COUNT_BITS: i32 = 6;
    const HISTORY_STATE_BITS: i32 = 12;

    const INVALID: Node = Node {
        packed: 0,
        children: [NodeChild::INVALID, NodeChild::INVALID],
    };

    fn new(text_start: usize, depth: usize,
           left_count: usize, right_count: usize, history_state: u32,
           children: [NodeChild; 2]) -> Node {
        assert!((text_start as u64) < 1u64 << Node::TEXT_START_BITS);
        assert!((depth as u64) < 1u64 << Node::DEPTH_BITS);
        assert!((left_count as u64) < 1u64 << Node::LEFT_COUNT_BITS);
        assert!((right_count as u64) < 1u64 << Node::RIGHT_COUNT_BITS);
        assert!((history_state as u64) < 1u64 << Node::HISTORY_STATE_BITS);
        let mut packed: u64 = 0;
        packed += text_start as u64;
        packed <<= Node::DEPTH_BITS;
        packed += depth as u64;
        packed <<= Node::LEFT_COUNT_BITS;
        packed += left_count as u64;
        packed <<= Node::RIGHT_COUNT_BITS;
        packed += right_count as u64;
        packed <<= Node::HISTORY_STATE_BITS;
        packed += history_state as u64;
        Node { packed, children }
    }

    fn is_valid(&self) -> bool {
        self.children[0] != NodeChild::INVALID &&
            self.children[1] != NodeChild::INVALID
    }

    fn text_start(&self) -> usize {
        let mut result = self.packed >> (9 + 6 + 6 + 12);
        result &= (1u64 << Node::TEXT_START_BITS) - 1;
        result as usize
    }

    fn depth(&self) -> usize {
        let mut result = self.packed >> (6 + 6 + 12);
        result &= (1u64 << Node::DEPTH_BITS) - 1;
        result as usize
    }

    fn left_count(&self) -> usize {
        let mut result = self.packed >> (6 + 12);
        result &= (1u64 << Node::LEFT_COUNT_BITS) - 1;
        result as usize
    }

    fn right_count(&self) -> usize {
        let mut result = self.packed >> 12;
        result &= (1u64 << Node::RIGHT_COUNT_BITS) - 1;
        result as usize
    }

    fn history_state(&self) -> u32 {
        let mut result = self.packed;
        result &= (1u64 << Node::HISTORY_STATE_BITS) - 1;
        result as u32
    }

    fn child(&self, direction: Direction) -> NodeChild {
        self.children[direction]
    }

    fn increment_edge_counters(&mut self, direction: Direction) {
        *self = Node::new(
            self.text_start(),
            self.depth(),
            direction.fold(|| 63.min(self.left_count() + 1),
                           || self.left_count()),
            direction.fold(|| self.right_count(),
                           || 63.min(self.right_count() + 1)),
            updated_bit_history(self.history_state(),
                                direction.fold(|| false, || true)),
            self.children,
        );
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
