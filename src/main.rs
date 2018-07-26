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

use std::io::Read;
use std::time::SystemTime;

use demixer::bit::Bit;
use demixer::fixed_point::FixedPoint;
use demixer::lut::LookUpTables;
use demixer::predictor::Predictor;
use demixer::predictor::stats::PredictionStatisticsType;

fn main() {
    print_banner();
    let args: Vec<String> = std::env::args().collect();
    let file_name = args.get(1).expect("provide file name");
    estimate_compression(file_name).unwrap();
}

fn print_banner() {
    eprintln!("demixer - file compressor aimed at high compression ratios");
    eprint!("Copyright (C) 2018  Piotr Tarsa ");
    eprintln!("( https://github.com/tarsa )");
    eprintln!();
}

fn estimate_compression(file_name: &String) -> std::io::Result<()> {
    let file = std::fs::File::open(file_name).expect("file not found");
    let start_time = SystemTime::now();
    let luts = LookUpTables::new();
    let mut predictor = Predictor::new(&luts);
    let initialization_secs = {
        let duration = SystemTime::now().duration_since(start_time).unwrap();
        duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9
    };

    let mut total_cost_in_bits = 0f64;
    let mut total_bytes = 0u64;

    for byte_read_result in std::io::BufReader::new(file).bytes() {
        let input_byte = byte_read_result?;
        predictor.start_new_byte();
        for bit_index in (0..=7).rev() {
            let prediction = predictor.predict();
            let input_bit: Bit = ((input_byte & (1 << bit_index)) != 0).into();
            predictor.update(input_bit);

            let prediction = prediction.as_f64();
            if input_bit.is_0() {
                total_cost_in_bits -= prediction.log2();
            } else {
                total_cost_in_bits -= (1.0 - prediction).log2();
            }
        }
        total_bytes += 1;
    }

    let total_duration_secs = {
        let duration = SystemTime::now().duration_since(start_time).unwrap();
        duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9
    };
    let speed_bps = total_bytes as f64 / total_duration_secs;
    let speed_ns_byte = total_duration_secs / total_bytes as f64 * 1e9;
    let average_bits_per_byte = total_cost_in_bits / total_bytes as f64;
    let compression_ratio = (total_bytes * 8) as f64 / total_cost_in_bits;

    println!("initialization         = {:15.3} seconds", initialization_secs);
    println!("total duration         = {:15.3} seconds", total_duration_secs);
    println!("speed                  = {:15.3} ns per byte", speed_ns_byte);
    println!("speed                  = {:15.3} bytes per second", speed_bps);
    println!("                       :  |  |  |  |   |");
    println!("total cost in bytes    = {:15.3}", total_cost_in_bits / 8.0);
    println!("total cost in bits     = {:15.3}", total_cost_in_bits);
    println!("input length           = {:11} bytes", total_bytes);
    println!("average cost           = {:17.5} bpb", average_bits_per_byte);
    println!("compression ratio      = {:15.3} : 1", compression_ratio);

    predictor.print_state(&[
        PredictionStatisticsType::AverageContextLength,
//        PredictionStatisticsType::CostsAndOccurrencesPerContextType,
//        PredictionStatisticsType::CostsAndOccurrencesPerSymbolValue,
        PredictionStatisticsType::TotalCostUsingLuts,
    ]);

    Result::Ok(())
}
