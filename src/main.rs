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

trait HistorySource {
    fn new(input_size: usize, max_order: usize) -> Self;

    fn start_new_byte(&mut self);

    fn gather_history_states(&self, bit_histories: &mut CollectedBitHistories);

    fn process_input_bit(&mut self, input_bit: bool);
}

struct NaiveHistorySource {
    input: Vec<u8>,
    input_cursor: usize,
    bit_index: usize,
    max_order: usize,
}

impl HistorySource for NaiveHistorySource {
    fn new(input_size: usize, max_order: usize) -> NaiveHistorySource {
        NaiveHistorySource {
            input: Vec::with_capacity(input_size),
            input_cursor: 0,
            bit_index: 7,
            max_order,
        }
    }

    fn start_new_byte(&mut self) {
        assert_eq!(self.bit_index, 7);
        assert_eq!(self.input_cursor, self.input.len());
        assert_ne!(self.input.len(), self.input.capacity());
        self.input.push(0);
    }

    fn gather_history_states(&self, bit_histories: &mut CollectedBitHistories) {
        for order in 0..(self.max_order + 1) {
            let mut bit_history = 1;
            for scanned_index in 0..(self.input_cursor - order) {
                let prefix_equal = compare_for_equal_prefix(
                    &self.input, scanned_index, self.input_cursor - order,
                    self.bit_index as i32, order,
                );
                if prefix_equal {
                    let next_bit = get_bit(self.input[scanned_index + order],
                                           self.bit_index as i32);
                    bit_history = updated_bit_history(bit_history, next_bit);
                }
            }
            if bit_history == 1 {
                break;
            }
            bit_histories.items.push(bit_history);
        }
    }

    fn process_input_bit(&mut self, input_bit: bool) {
        self.input[self.input_cursor] |= (input_bit as u8) << self.bit_index;
        if self.bit_index > 0 {
            self.bit_index -= 1;
        } else {
            self.bit_index = 7;
            self.input_cursor += 1;
        }
    }
}

#[derive(Clone)]
struct ContextState {
    byte_index: usize,
    bit_history: u32,
}

struct FatMapHistorySource {
    input: Vec<u8>,
    input_cursor: usize,
    bit_index: usize,
    max_order: usize,
    maps: Vec<HashMap<u64, Vec<ContextState>>>,
}

impl FatMapHistorySource {
    fn compute_hash(&self, order: usize) -> u64 {
        let map = &self.maps[(order * 8) + (self.bit_index as usize)];
        let mut hasher: DefaultHasher = map.hasher().build_hasher();
        hasher.write(
            &self.input[self.input_cursor - order..self.input_cursor]);
        hasher.write_u32((256 + self.input[self.input_cursor] as u32) >>
            (self.bit_index + 1));
        hasher.finish()
    }
}

impl HistorySource for FatMapHistorySource {
    fn new(input_size: usize, max_order: usize) -> FatMapHistorySource {
        FatMapHistorySource {
            input: Vec::with_capacity(input_size),
            input_cursor: 0,
            bit_index: 7,
            max_order,
            maps: vec![HashMap::new(); (max_order + 1) * 8],
        }
    }

    fn start_new_byte(&mut self) {
        assert_eq!(self.bit_index, 7);
        assert_eq!(self.input_cursor, self.input.len());
        assert_ne!(self.input.len(), self.input.capacity());
        self.input.push(0);
    }

    fn gather_history_states(&self, bit_histories: &mut CollectedBitHistories) {
        for order in 0..(self.max_order.min(self.input_cursor) + 1) {
            let map = &self.maps[(order * 8) + (self.bit_index as usize)];
            let hash = self.compute_hash(order);
            let vec_opt: Option<&Vec<_>> = map.get(&hash);
            match vec_opt.into_iter().
                flat_map(|vec| vec.into_iter().find(|item| {
                    compare_for_equal_prefix(
                        &self.input, self.input_cursor - order,
                        item.byte_index, self.bit_index as i32, order)
                })).last() {
                Some(ctx) => bit_histories.items.push(ctx.bit_history),
                None => break,
            };
        }
    }

    fn process_input_bit(&mut self, input_bit: bool) {
        for order in 0..(self.max_order.min(self.input_cursor) + 1) {
            let hash = self.compute_hash(order);
            let map = &mut self.maps[(order * 8) + (self.bit_index as usize)];
            let vec: &mut Vec<_> = map.entry(hash).or_insert(Vec::new());
            let input = &self.input;
            let byte_index = self.input_cursor - order;
            let bit_index = self.bit_index;
            let found = vec.iter_mut().find(|item| compare_for_equal_prefix(
                input, byte_index, item.byte_index, bit_index as i32, order)
            ).map(|ctx| ctx.bit_history =
                updated_bit_history(ctx.bit_history, input_bit as u8)
            ).is_some();
            if !found {
                vec.push(ContextState {
                    byte_index,
                    bit_history: 2 + input_bit as u32,
                });
            };
        }
        self.input[self.input_cursor] |= (input_bit as u8) << self.bit_index;
        if self.bit_index > 0 {
            self.bit_index -= 1;
        } else {
            self.bit_index = 7;
            self.input_cursor += 1;
        }
    }
}

fn main() {
    print_banner();

    let args: Vec<String> = std::env::args().collect();
    let history_source_type: &str = args.get(1).expect("provide type");
    let file_name = args.get(2).expect("provide file name");

    let mut file = std::fs::File::open(file_name).expect("file not found");
//    for byte in std::io::BufReader::new(file).bytes() {}
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    std::mem::drop(file);

    match history_source_type {
        "brute_force" =>
            print_bit_histories::<NaiveHistorySource>(&buffer),
        "fat_map" =>
            print_bit_histories::<FatMapHistorySource>(&buffer),
        "tree" =>
            print_bit_histories::<tree::TreeHistorySource>(&buffer),
        _ =>
            panic!("unrecognized history source type!")
    }
}

fn print_banner() {
    eprintln!("demixer - file compressor aimed at high compression ratios");
    eprint!("Copyright (C) 2018  Piotr Tarsa ");
    eprintln!(" https://github.com/tarsa )");
    eprintln!();
}

fn print_bit_histories<Source: HistorySource>(input: &[u8]) {
    let mut collected_states =
        CollectedBitHistories::new(MAX_ORDER);
    let mut history_source =
        Source::new(input.len(), MAX_ORDER);
    for (i, &x) in input.iter().take(1234).enumerate() {
        println!("Processing byte with index: {}, {}", i, x as char);
        history_source.start_new_byte();
        for bit_index in (0..7 + 1).rev() {
            collected_states.reset();
            history_source.gather_history_states(&mut collected_states);
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
            history_source.process_input_bit(incoming_bit);
        }
        println!();
    }
}
