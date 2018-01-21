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

use core::hash::BuildHasher;
use core::hash::Hasher;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::io::prelude::*;

mod tree;

pub const MAX_ORDER: usize = 63;

fn updated_bit_history(bit_history: u32, next_bit: u8) -> u32 {
    ((bit_history << 1) & 2047) | (next_bit as u32) | (bit_history & 1024)
}

fn get_bit(byte: u8, bit_index: i32) -> u8 {
    (byte >> bit_index) & 1
}

fn bytes_differ_on(contents: &[u8], first_byte_index: usize,
                   second_byte_index: usize, bit_index: i32) -> bool {
    get_bit(contents[first_byte_index] ^ contents[second_byte_index],
            bit_index) == 1
}

fn compare_for_equal_prefix(contents: &[u8], starting_index_first: usize,
                            starting_index_second: usize, bit_index: i32,
                            full_byte_length: usize) -> bool {
    let mut equal = true;
    for position in 0..full_byte_length {
        equal &= contents[starting_index_first + position] ==
            contents[starting_index_second + position];
        if !equal { break };
    }
    let mut bit_position = 7;
    while equal && bit_position > bit_index {
        equal &= !bytes_differ_on(contents,
                                  starting_index_first + full_byte_length,
                                  starting_index_second + full_byte_length,
                                  bit_position);
        bit_position -= 1;
    }
    equal
}

fn brute_force_process_bit(contents: &[u8], byte_index: usize, bit_index: i32) {
    let mut something_printed = false;
    for order in 0..MAX_ORDER + 1 {
        let mut bit_history = 1;
        for scanned_index in 0..byte_index - order {
            if compare_for_equal_prefix(contents, scanned_index,
                                        byte_index - order, bit_index, order) {
                bit_history = updated_bit_history(
                    bit_history,
                    get_bit(contents[scanned_index + order], bit_index));
            }
        }
        if bit_history == 1 {
            break;
        }
        if !something_printed {
            print!("{}: ", bit_index);
        }
        if order != 0 {
            print!(", ");
        }
        print!("{:x}", bit_history);
        something_printed = true;
    }
    if something_printed {
        println!();
    }
}

#[derive(Clone)]
struct ContextState {
    byte_index: usize,
    bit_history: u32,
}

fn fat_map_process_bit(contents: &[u8],
                       maps: &mut Vec<HashMap<u64, Vec<ContextState>>>,
                       byte_index: usize, bit_index: i32) {
    let mut something_printed = false;
    let mut silent = false;
    let bit = get_bit(contents[byte_index], bit_index);
    for order in 0..MAX_ORDER.min(byte_index) + 1 {
        let map = &mut maps[(order * 8) + (bit_index as usize)];
        let mut hasher: DefaultHasher = map.hasher().build_hasher();
        hasher.write(&contents[byte_index - order..byte_index]);
        hasher.write_u32(
            (256 + contents[byte_index] as u32) >> (bit_index + 1));
        let hash = hasher.finish();
        let vec: &mut Vec<_> = map.entry(hash).or_insert_with(|| Vec::new());
        let bit_history = match vec.iter_mut().find(|item|
            compare_for_equal_prefix(contents, byte_index - order,
                                     item.byte_index, bit_index, order)) {
            Some(ctx) => {
                let result = ctx.bit_history;
                ctx.bit_history = updated_bit_history(result, bit);
                result
            }
            None =>
                1
        };
        if bit_history == 1 {
            let fresh = ContextState {
                byte_index: byte_index - order,
                bit_history: 2 + bit as u32,
            };
            vec.push(fresh);
            silent = true;
        }
        if !silent {
            if !something_printed {
                print!("{}: ", bit_index);
            }
            if order != 0 {
                print!(", ");
            }
            print!("{:x}", bit_history);
            something_printed = true;
        }
    }
    if something_printed {
        println!();
    }
}

fn main() {
    print_banner();

    let args: Vec<String> = std::env::args().collect();
    let history_source_type: &str = args.get(1).expect("provide type");
    let file_name = args.get(2).expect("provide file name");

    let mut file = std::fs::File::open(file_name).expect("file not found");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    std::mem::drop(file);

    match history_source_type {
        "brute_force" =>
            for (i, &x) in buffer.iter().take(1234).enumerate() {
                println!("Processing byte with index: {}, {}", i, x as char);
                for b in (0..7 + 1).rev() {
                    brute_force_process_bit(&buffer, i, b);
                }
                println!();
            },
        "fat_map" => {
            let mut maps = vec![HashMap::new(); (MAX_ORDER + 1) * 8];
            for (i, &x) in buffer.iter().take(1234).enumerate() {
                println!("Processing byte with index: {}, {}", i, x as char);
                for b in (0..7 + 1).rev() {
                    fat_map_process_bit(&buffer, &mut maps, i, b);
                }
                println!();
            }
        }
        "tree" => {
            let mut collected_states =
                tree::CollectedBitHistories::new(MAX_ORDER);
            let nodes = tree::Nodes::new(buffer.len());
            let mut tree = tree::Tree::new(nodes, buffer.len(), 0);
            let mut active_contexts = tree::ActiveContexts::new(MAX_ORDER);
            for (i, &x) in buffer.iter().take(1234).enumerate() {
                println!("Processing byte with index: {}, {}", i, x as char);
                for bit_index in (0..7 + 1).rev() {
                    tree.gather_states(&active_contexts, &mut collected_states,
                                       bit_index);
                    if collected_states.items.len() > 0 {
                        print!("{}: ", bit_index);
                        print!("{:x}", collected_states.items[0]);
                        for i in 1..collected_states.items.len() {
                            print!(", ");
                            print!("{:x}", collected_states.items[i]);
                        }
                        println!();
                    }
                    let incoming_bit = get_bit(x, bit_index as i32) == 1;
                    tree.extend(&mut active_contexts, incoming_bit,
                                bit_index, MAX_ORDER);
                }
                active_contexts.shift(&tree);
                tree.window_cursor += 1;
                println!();
            }
        }
        _ => panic!("unrecognized history source type!")
    }
}

fn print_banner() {
    eprintln!("demixer - file compressor aimed at high compression ratios");
    eprint!("Copyright (C) 2018  Piotr Tarsa ");
    eprintln!(" https://github.com/tarsa )");
    eprintln!();
}
