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
extern crate demixer;

use std::io::prelude::*;

use demixer::MAX_ORDER;
use demixer::history::{CollectedContextStates, HistorySource};
use demixer::history::naive::NaiveHistorySource;
use demixer::history::fat_map::FatMapHistorySource;
use demixer::history::tree::TreeHistorySource;
use demixer::history::window::get_bit;
use demixer::lut::LookUpTables;

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

    let luts = LookUpTables::new();
    match history_source_type {
        "brute_force" =>
            print_bit_histories::<NaiveHistorySource>(&buffer, &luts),
        "fat_map" =>
            print_bit_histories::<FatMapHistorySource>(&buffer, &luts),
        "tree" =>
            print_bit_histories::<TreeHistorySource>(&buffer, &luts),
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

fn print_bit_histories<'a, Source: HistorySource<'a>>(input: &[u8],
                                                      luts: &'a LookUpTables) {
    let mut collected_states = CollectedContextStates::new(MAX_ORDER);
    let mut history_source = Source::new(input.len(), MAX_ORDER, &luts);
    for (i, &x) in input.iter().take(1234).enumerate() {
        println!("Processing byte with index: {}, {}", i, x as char);
        history_source.start_new_byte();
        for bit_index in (0..7 + 1).rev() {
            collected_states.reset();
            history_source.gather_history_states(&mut collected_states);
            if collected_states.items().len() > 0 {
                print!("{}: ", bit_index);
                print!("{:x}", collected_states.items()[0].bit_history());
                for i in 1..collected_states.items().len() {
                    print!(", ");
                    print!("{:x}", collected_states.items()[i].bit_history());
                }
                println!();
            }
            let incoming_bit = get_bit(x, bit_index);
            history_source.process_input_bit(incoming_bit);
        }
        println!();
    }
}
