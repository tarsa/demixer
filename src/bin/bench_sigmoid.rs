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

use demixer::fixed_point::FixedPoint;
use demixer::fixed_point::types::StretchedProbD;
use demixer::lut::LookUpTables;
use demixer::random::MersenneTwister;
use std::time::SystemTime;

fn main() {
    let elems = 1000;
    let warm_up_loops = 10_000;
    let loops = 10_000;
    let luts = LookUpTables::new();
    let mut prng = MersenneTwister::default();
    let stretched_probs: Vec<_> = (0..elems).into_iter()
        .map(|_| generate_stretched_prob(&mut prng)).collect();
    let squashed_probs: Vec<_> = stretched_probs.iter()
        .map(|prob| luts.squash_lut().squash(prob.clone())).collect();
    let mut sum_warmup = 0u64;
    for _ in 0..warm_up_loops {
        for prob in stretched_probs.iter() {
            sum_warmup += luts.squash_lut().squash(prob.clone()).raw() as u64;
        }
    }
    let start_time_squash = SystemTime::now();
    let mut sum_squash = 0u64;
    for _ in 0..loops {
        for prob in stretched_probs.iter() {
            sum_squash += luts.squash_lut().squash(prob.clone()).raw() as u64;
        }
    }
    let end_time_squash = SystemTime::now();
    let start_time_stretch = SystemTime::now();
    let mut sum_stretch = 0i64;
    for _ in 0..loops {
        for prob in squashed_probs.iter() {
            sum_stretch +=
                luts.stretch_lut().stretch(prob.clone()).raw() as i64;
        }
    }
    let end_time_stretch = SystemTime::now();
    let nanos_per_squash = {
        let duration =
            end_time_squash.duration_since(start_time_squash).unwrap();
        let nanos =
            duration.as_secs() as f64 * 1e9 + duration.subsec_nanos() as f64;
        nanos / (elems as f64 * loops as f64)
    };
    let nanos_per_stretch = {
        let duration =
            end_time_stretch.duration_since(start_time_stretch).unwrap();
        let nanos =
            duration.as_secs() as f64 * 1e9 + duration.subsec_nanos() as f64;
        nanos / (elems as f64 * loops as f64)
    };
    println!("warmup (checksum: {})", sum_warmup);
    println!("squash  = {:5.1}ns (chksum: {})\nstretch = {:5.1}ns (chksum: {})",
             nanos_per_squash, sum_squash, nanos_per_stretch, sum_stretch);
}

fn generate_stretched_prob(prng: &mut MersenneTwister) -> StretchedProbD {
    assert_eq!(StretchedProbD::ABSOLUTE_LIMIT, 12);
    let unscaled = prng.next_int64() as i32;
    if unscaled < (-12 << 32 - 5) || unscaled > (12 << 32 - 5) {
        generate_stretched_prob(prng)
    } else {
        let fract_bits = StretchedProbD::FRACTIONAL_BITS;
        StretchedProbD::new(unscaled >> 32 - 5 - fract_bits, fract_bits)
    }
}
