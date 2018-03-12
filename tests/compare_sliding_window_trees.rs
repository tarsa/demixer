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
extern crate demixer;

use demixer::PRINT_DEBUG;
use demixer::history::{CollectedContextStates, HistorySource};
use demixer::history::tree::{Tree, TreeHistorySource, TreeState};
use demixer::history::tree::direction::Direction;

#[test]
fn compare_for_repeated_byte_input() {
    let lengths_of_inputs_and_prefixes = vec![
        (0, 7, 13),
        (2, 0, 13),
        (1, 13, 28),
        (22, 54, 83),
    ];
    for &byte in [0xb5, ' ' as u8, 'a' as u8].iter() {
        for &(prefix_1_length, prefix_2_length, common_input_length) in
            lengths_of_inputs_and_prefixes.iter() {
            let prefix_1 = vec![byte; prefix_1_length];
            let prefix_2 = vec![byte; prefix_2_length];
            let common_input = vec![byte; common_input_length];
            for &max_window_size in [0, 1, 2, 3, 4, 7, 20, 40, 100]
                .iter().take_while(|&&size| size < common_input_length) {
                for &max_order in [0, 1, 2, 3, 4, 7, 20, 40, 100]
                    .iter().take_while(|&&order| order < max_window_size) {
                    compare_for_input(&prefix_1, &prefix_2, &common_input,
                                      max_window_size, max_order);
                }
            }
        }
    }
}

#[test]
fn compare_for_two_symbols_interrupted_runs() {
    let symbols_pairs: &[(u8, u8)] =
        &[(0, 255), ('b' as u8, 'a' as u8), (215, 15), (31, 32)];
    let lengths_of_inputs_and_prefixes = vec![
        (0, 7, 13),
        (2, 0, 13),
        (1, 13, 28),
        (22, 54, 83),
    ];
    for &(sym_0, sym_1) in symbols_pairs.iter() {
        for &interruption_period in [2, 3, 4, 7, 10].iter() {
            let mut input0 = vec![sym_0; interruption_period];
            input0.push(sym_1);
            let mut input = input0.clone();
            while input.len() < 300 {
                input.append(&mut input0.clone());
            }
            for &(prefix_1_length, prefix_2_length, common_input_length) in
                lengths_of_inputs_and_prefixes.iter() {
                let prefix_1 = &input[..prefix_1_length];
                let prefix_2 = &input[..prefix_2_length];
                let common_input = &input[..common_input_length];
                for &max_window_size in [0, 1, 2, 3, 4, 7, 20, 40, 100]
                    .iter().take_while(|&&size| size < common_input_length) {
                    for &max_order in [0, 1, 2, 3, 4, 7, 20, 40, 100]
                        .iter().take_while(|&&order| order < max_window_size) {
                        compare_for_input(prefix_1, prefix_2, common_input,
                                          max_window_size, max_order);
                    }
                }
            }
        }
    }
}

#[test]
fn compare_for_two_symbols_fibonacci_word() {
    let symbols_pairs: &[(u8, u8)] =
        &[(0, 255), ('b' as u8, 'a' as u8), (215, 15), (31, 32)];
    let lengths_of_inputs_and_prefixes = vec![
        (0, 7, 13),
        (2, 0, 13),
        (1, 13, 28),
        (22, 54, 83),
    ];
    for &(sym_0, sym_1) in symbols_pairs.iter() {
        let mut input0 = vec![sym_0];
        let mut input = vec![sym_0, sym_1];
        while input.len() < 300 {
            let old_word1 = input.clone();
            input.append(&mut input0);
            input0 = old_word1;
        }
        for &(prefix_1_length, prefix_2_length, common_input_length) in
            lengths_of_inputs_and_prefixes.iter() {
            let prefix_1 = &input[..prefix_1_length];
            let prefix_2 = &input[..prefix_2_length];
            let common_input = &input[..common_input_length];
            for &max_window_size in [0, 1, 2, 3, 4, 7, 20, 40, 100]
                .iter().take_while(|&&size| size < common_input_length) {
                for &max_order in [0, 1, 2, 3, 4, 7, 20, 40, 100]
                    .iter().take_while(|&&order| order < max_window_size) {
                    compare_for_input(prefix_1, prefix_2, common_input,
                                      max_window_size, max_order);
                }
            }
        }
    }
}

#[test]
fn compare_for_multi_symbol_sequences() {
    let lengths_of_inputs_and_prefixes = vec![
        (0, 7, 13),
        (2, 0, 13),
        (1, 13, 28),
        (22, 54, 83),
    ];
    for &starting_symbol in [0 as u8, 'a' as u8].iter() {
        let mut input = vec![starting_symbol];
        let mut next_symbol = starting_symbol + 1;
        while input.len() < 300 {
            let mut clone = input.clone();
            input.append(&mut clone);
            input.push(next_symbol);
            next_symbol += 1;
        }
        for &(prefix_1_length, prefix_2_length, common_input_length) in
            lengths_of_inputs_and_prefixes.iter() {
            let prefix_1 = &input[..prefix_1_length];
            let prefix_2 = &input[..prefix_2_length];
            let common_input = &input[..common_input_length];
            for &max_window_size in [0, 1, 2, 3, 4, 7, 20, 40, 100]
                .iter().take_while(|&&size| size < common_input_length) {
                for &max_order in [0, 1, 2, 3, 4, 7, 20, 40, 100]
                    .iter().take_while(|&&order| order < max_window_size) {
                    compare_for_input(prefix_1, prefix_2, common_input,
                                      max_window_size, max_order);
                }
            }
        }
    }
}

#[test]
fn compare_for_repeated_byte_borders() {
    let lengths_of_prefixes_and_suffixes = vec![
        (1, 7, 13),
        (2, 1, 13),
        (20, 13, 28),
        (22, 54, 83),
    ];
    let border_and_middle_starter_symbols: &[(u8, u8)] =
        &[(0, 128), ('z' as u8, 'a' as u8), (215, 15), (31, 32)];
    for &(border_sym, middle_sym) in border_and_middle_starter_symbols.iter() {
        let mut middle = vec![middle_sym];
        let mut next_symbol: u8 = middle_sym + 1;
        while middle.len() < 300 {
            let mut clone = middle.clone();
            middle.append(&mut clone);
            middle.push(next_symbol);
            next_symbol += 1;
        }
        for &(prefix_1_length, prefix_2_length, suffix_length) in
            lengths_of_prefixes_and_suffixes.iter() {
            let prefix_1 = vec![border_sym; prefix_1_length];
            let prefix_2 = vec![border_sym; prefix_2_length];
            for &middle_length in [0, 1, 2, 3, 4, 7, 20, 40, 100].iter() {
                let mut common_input: Vec<_> = middle[..middle_length].to_vec();
                common_input.append(&mut vec![border_sym; suffix_length +
                    prefix_1_length.min(prefix_2_length) + 1]);
                let max_window_size = middle_length + suffix_length;
                for &max_order in [0, 1, 2, 3, 4, 7, 20, 40, 100]
                    .iter().take_while(|&&order| order < max_window_size) {
                    compare_for_input(&prefix_1, &prefix_2, &common_input,
                                      max_window_size, max_order);
                }
            }
        }
        assert!(middle.len() >= 300);
    }
}

#[test]
fn compare_for_repeated_pattern_borders() {
    let lengths_of_prefixes_and_suffixes = vec![
        (1, 7, 13),
        (2, 1, 13),
        (20, 13, 28),
        (22, 54, 83),
    ];
    let border_and_middle_starter_symbols: &[(u8, u8, u8)] =
        &[(0, 255, 128), ('z' as u8, 'v' as u8, 'a' as u8), (31, 32, 215)];
    for &(border_sym_0, border_sym_1, middle_sym)
        in border_and_middle_starter_symbols.iter() {
        let mut middle = vec![middle_sym];
        let mut next_symbol: u8 = middle_sym + 1;
        while middle.len() < 300 {
            let mut clone = middle.clone();
            middle.append(&mut clone);
            middle.push(next_symbol);
            next_symbol += 1;
        }
        for &interruption_period in [1, 2, 3, 7].iter() {
            let mut border = vec![border_sym_0; interruption_period];
            border.push(border_sym_1);
            while border.len() < 300 {
                let mut clone = border.clone();
                border.append(&mut clone);
            }
            for &(prefix_1_length, prefix_2_length, suffix_length) in
                lengths_of_prefixes_and_suffixes.iter() {
                let prefix_1 = &border[..prefix_1_length];
                let prefix_2 = &border[..prefix_2_length];
                for &middle_length in [0, 1, 2, 3, 4, 7, 20, 40, 100].iter() {
                    let mut common_input = middle[..middle_length].to_vec();
                    common_input.extend_from_slice(&border[..suffix_length +
                        prefix_1_length.min(prefix_2_length) + 1]);
                    let max_window_size = middle_length + suffix_length;
                    for &max_order in [0, 1, 2, 3, 4, 7, 20, 40, 100]
                        .iter().take_while(|&&order| order < max_window_size) {
                        compare_for_input(&prefix_1, &prefix_2, &common_input,
                                          max_window_size, max_order);
                    }
                }
            }
        }
        assert!(middle.len() >= 300);
    }
}

fn compare_for_input(prefix_1: &[u8], prefix_2: &[u8], common: &[u8],
                     max_window_size: usize, max_order: usize) {
    assert!(max_order < max_window_size);

    let offset_1 = prefix_1.len() % max_window_size;
    let offset_2 = prefix_2.len() % max_window_size;

    if PRINT_DEBUG {
        println!("COMPARE SLIDING WINDOW TREES: max order = {}, \
                  max window size = {}", max_order, max_window_size);
        println!("prefix 1 = {:?}\nprefix 2 = {:?}\ncommon = {:?}\n",
                 prefix_1, prefix_2, common);
    }
    assert!(common.len() > max_window_size,
            "convergence can only be checked after \
             windows' contents are identical");

    let mut source_1 = TreeHistorySource::new(max_window_size, max_order);
    let mut source_2 = TreeHistorySource::new(max_window_size, max_order);
    let mut source_1_results = CollectedContextStates::new(max_order);
    let mut source_2_results = CollectedContextStates::new(max_order);

    if PRINT_DEBUG { println!("FILLING UP tree 1 with prefix data"); }
    for (index, byte) in prefix_1.iter().enumerate() {
        source_1.check_integrity_before_next_byte();
        source_1.start_new_byte();
        if PRINT_DEBUG {
            println!("prefix 1: started byte #{}, max order = {}, \
                      max window size = {}, input = {:?}",
                     index, max_order, max_window_size, &prefix_1[..index + 1]);
            source_1.tree.print();
        }
        for bit_index in (0..7 + 1).rev() {
            verify_live_nodes_count(&source_1.tree);
            if PRINT_DEBUG {
                println!("active contexts 1 = {}", source_1.active_contexts);
                println!("before: index = {}, bit index = {}, prefix 1 = {:?}",
                         index, bit_index, &prefix_1[..index + 1]);
            }

            let input_bit = (byte & (1 << bit_index)) != 0;
            if PRINT_DEBUG { println!("processing bit: {}", input_bit); }
            source_1.process_input_bit(input_bit);
            if PRINT_DEBUG { println!(); }
        }
    }
    if PRINT_DEBUG { println!(); }

    if PRINT_DEBUG { println!("FILLING UP tree 2 with prefix data"); }
    for (index, byte) in prefix_2.iter().enumerate() {
        source_2.check_integrity_before_next_byte();
        source_2.start_new_byte();
        if PRINT_DEBUG {
            println!("prefix 2: started byte #{}, max order = {}, \
                      max window size = {}, input = {:?}",
                     index, max_order, max_window_size, &prefix_2[..index + 1]);
            source_2.tree.print();
        }
        for bit_index in (0..7 + 1).rev() {
            verify_live_nodes_count(&source_2.tree);
            if PRINT_DEBUG {
                println!("active contexts 2 = {}", source_2.active_contexts);
                println!("before: index = {}, bit index = {}, prefix 2 = {:?}",
                         index, bit_index, &prefix_2[..index + 1]);
            }

            let input_bit = (byte & (1 << bit_index)) != 0;
            if PRINT_DEBUG { println!("processing bit: {}", input_bit); }
            source_2.process_input_bit(input_bit);
            if PRINT_DEBUG { println!(); }
        }
    }
    if PRINT_DEBUG { println!(); }

    if PRINT_DEBUG {
        println!("before converging");
        println!("tree 1");
        source_1.tree.print();
        println!("tree 2");
        source_2.tree.print();
        println!();
    }

    if PRINT_DEBUG { println!("CONVERGING both trees"); }
    for (index, byte) in common[..max_window_size].iter().enumerate() {
        if PRINT_DEBUG {
            println!("CONVERGING: started byte #{}, max order = {}, \
                      max window size = {}",
                     index, max_order, max_window_size);
        }
        if PRINT_DEBUG {
            println!("source 1, prefix = {:?}, common = {:?}",
                     prefix_1, &common[..index + 1]);
            source_1.tree.print();
        }
        source_1.check_integrity_before_next_byte();
        source_1.start_new_byte();
        if PRINT_DEBUG {
            println!("source 2, prefix = {:?}, common = {:?}",
                     prefix_2, &common[..index + 1]);
            source_2.tree.print();
        }
        source_2.check_integrity_before_next_byte();
        source_2.start_new_byte();

        for bit_index in (0..7 + 1).rev() {
            verify_live_nodes_count(&source_1.tree);
            verify_live_nodes_count(&source_2.tree);
            if PRINT_DEBUG {
                println!("active contexts 1 = {}", source_1.active_contexts);
                println!("active contexts 2 = {}", source_2.active_contexts);
                println!("before: index = {}, bit index = {}, input = {:?}",
                         index, bit_index, &common[..index + 1]);
            }

            let input_bit = (byte & (1 << bit_index)) != 0;
            if PRINT_DEBUG { println!("processing bit: {}", input_bit); }
            if PRINT_DEBUG { println!("source 1"); }
            source_1.process_input_bit(input_bit);
            if PRINT_DEBUG { println!("source 2"); }
            source_2.process_input_bit(input_bit);
            if PRINT_DEBUG { println!(); }
        }
    }
    if PRINT_DEBUG { println!(); }

    if PRINT_DEBUG {
        println!("after converging");
        println!("tree 1");
        source_1.tree.print();
        println!("tree 2");
        source_2.tree.print();
        println!();
    }

    if PRINT_DEBUG { println!("VERIFYING SIMILARITY BETWEEN SOURCES now"); }
    for (index, byte) in common[max_window_size..].iter().enumerate() {
        let index = index + max_window_size;

        if PRINT_DEBUG {
            println!("VERIFYING: started byte #{}, max order = {}, \
                      max window size = {}",
                     index, max_order, max_window_size);
        }
        if PRINT_DEBUG {
            println!("source 1, prefix = {:?}, common = {:?}",
                     prefix_1, &common[..index + 1]);
            source_1.tree.print();
        }
        source_1.check_integrity_before_next_byte();
        source_1.start_new_byte();
        if PRINT_DEBUG {
            println!("source 2, prefix = {:?}, common = {:?}",
                     prefix_2, &common[..index + 1]);
            source_2.tree.print();
        }
        source_2.check_integrity_before_next_byte();
        source_2.start_new_byte();

        for bit_index in (0..7 + 1).rev() {
            compare_shape(offset_1, &source_1.tree, offset_2, &source_2.tree);
            source_1_results.reset();
            source_1.gather_history_states(&mut source_1_results);
            source_2_results.reset();
            source_2.gather_history_states(&mut source_2_results);

            if PRINT_DEBUG {
                println!("active contexts 1 = {}", source_1.active_contexts);
                println!("active contexts 2 = {}", source_2.active_contexts);
                println!("before: index = {}, bit index = {}, input = {:?}",
                         index, bit_index, &common[..index + 1]);
            }

            assert_eq!(
                source_1.active_contexts.items().iter().map(
                    |ctx| ctx.prepare_for_test(offset_1, &source_1.tree.window)
                ).collect::<Vec<_>>(),
                source_2.active_contexts.items().iter().map(
                    |ctx| ctx.prepare_for_test(offset_2, &source_2.tree.window)
                ).collect::<Vec<_>>()
            );

            assert_eq!(
                source_1_results.items().iter().map(
                    |ctx_state| source_1.tree.window.index_subtract(
                        ctx_state.last_occurrence_index, offset_1)
                ).collect::<Vec<_>>(),
                source_2_results.items().iter().map(
                    |ctx_state| source_2.tree.window.index_subtract(
                        ctx_state.last_occurrence_index, offset_2))
                    .collect::<Vec<_>>(),
                "index = {}, bit index = {}, input = {:?}",
                index, bit_index, &common[..index + 1]);

            let input_bit = (byte & (1 << bit_index)) != 0;
            if PRINT_DEBUG { println!("processing bit: {}", input_bit); }
            if PRINT_DEBUG { println!("source 1"); }
            source_1.process_input_bit(input_bit);
            if PRINT_DEBUG { println!("source 2"); }
            source_2.process_input_bit(input_bit);
            if PRINT_DEBUG { println!(); }
        }
    }

    if PRINT_DEBUG { println!("SHRINKING BOTH SOURCES"); }
    for index in 0..max_window_size {
        if PRINT_DEBUG {
            println!("SHRINKING: started byte #{}, max order = {}, \
                      max window size = {}",
                     index, max_order, max_window_size);
        }
        if PRINT_DEBUG {
            println!("source 1, prefix = {:?}, common = {:?}",
                     prefix_1, &common[..index + 1]);
            source_1.tree.print();
        }
        source_1.check_integrity_before_next_byte();
        source_1.tree.remove_leftmost_suffix(&mut source_1.active_contexts);
        if PRINT_DEBUG {
            println!("source 2, prefix = {:?}, common = {:?}",
                     prefix_2, &common[..index + 1]);
            source_2.tree.print();
        }
        source_2.check_integrity_before_next_byte();
        source_2.tree.remove_leftmost_suffix(&mut source_2.active_contexts);

        compare_shape(offset_1, &source_1.tree, offset_2, &source_2.tree);

        if PRINT_DEBUG { println!(); }
    }

    assert_eq!(source_1.tree.window.size, 0);
    assert_eq!(source_2.tree.window.size, 0);
}

fn verify_live_nodes_count(tree: &Tree) {
    let mut indices_stack = Vec::new();

    let mut visited_nodes = 0;
    if tree.tree_state == TreeState::Proper {
        indices_stack.push(tree.get_root_node_index());
    }

    while let Some(node_index) = indices_stack.pop() {
        visited_nodes += 1;
        let node = &tree.nodes()[node_index];

        let left_child = node.child(Direction::Left);
        if left_child.is_node_index() {
            indices_stack.push(left_child.to_node_index());
        }

        let right_child = node.child(Direction::Right);
        if right_child.is_node_index() {
            indices_stack.push(right_child.to_node_index());
        }
    }

    assert_eq!(tree.nodes().live_nodes_count(), visited_nodes);
}

fn compare_shape(offset_1: usize, tree_1: &Tree,
                 offset_2: usize, tree_2: &Tree) {
    let mut stack_1 = Vec::new();
    let mut stack_2 = Vec::new();

    let mut visited_nodes_1 = 0;
    let mut visited_nodes_2 = 0;
    if tree_1.tree_state == TreeState::Proper {
        stack_1.push(tree_1.get_root_node_index());
    }
    if tree_2.tree_state == TreeState::Proper {
        stack_2.push(tree_2.get_root_node_index());
    }

    while !stack_1.is_empty() || !stack_2.is_empty() {
        assert!(!stack_1.is_empty() && !stack_2.is_empty());
        let node_index_1 = stack_1.pop().unwrap();
        let node_index_2 = stack_2.pop().unwrap();
        visited_nodes_1 += 1;
        visited_nodes_2 += 1;
        let node_1 = &tree_1.nodes()[node_index_1];
        let node_2 = &tree_2.nodes()[node_index_2];

        assert_eq!(node_1.depth(), node_2.depth());
        assert_eq!(tree_1.window.index_subtract(node_1.text_start(), offset_1),
                   tree_2.window.index_subtract(node_2.text_start(), offset_2));

        let node_1_left_child = node_1.child(Direction::Left);
        let node_1_right_child = node_1.child(Direction::Right);
        let node_2_left_child = node_2.child(Direction::Left);
        let node_2_right_child = node_2.child(Direction::Right);

        assert_eq!(node_1_left_child.is_window_index(),
                   node_2_left_child.is_window_index());
        assert_eq!(node_1_right_child.is_window_index(),
                   node_2_right_child.is_window_index());

        if node_1_left_child.is_node_index() {
            assert!(node_2_left_child.is_node_index());
            stack_1.push(node_1_left_child.to_node_index());
            stack_2.push(node_2_left_child.to_node_index());
        }
        if node_1_right_child.is_node_index() {
            assert!(node_2_right_child.is_node_index());
            stack_1.push(node_1_right_child.to_node_index());
            stack_2.push(node_2_right_child.to_node_index());
        }
    }

    assert_eq!(tree_1.nodes().live_nodes_count(),
               tree_2.nodes().live_nodes_count());
    assert_eq!(tree_1.nodes().live_nodes_count(), visited_nodes_1);
    assert_eq!(tree_2.nodes().live_nodes_count(), visited_nodes_2);
}
