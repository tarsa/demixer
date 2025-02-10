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

use demixer::random::MersenneTwister;
use demixer::util::permutation::{Permutation, PermutationBuilder};

#[test]
fn check_initial_state() {
    let permutation = PermutationBuilder::new().build();
    assert!(permutations_are_inverses(&permutation));
    assert!((0u8..=255u8).all(|value| {
        permutation.forward(value) == value &&
            permutation.backward(value) == value
    }));
}

#[test]
fn check_simple_swaps() {
    let mut builder = PermutationBuilder::new();
    builder
        .swap_pair(5, 10)
        .swap_segment(20, 160, 50);
    let permutation = builder.build();
    assert!(permutations_are_inverses(&permutation));
    assert_eq!(permutation.forward(5), 10);
    assert_eq!(permutation.forward(10), 5);
    assert_eq!(permutation.forward(20), 160);
    assert_eq!(permutation.forward(160), 20);
    assert_eq!(permutation.forward(40), 180);
    assert_eq!(permutation.forward(180), 40);
}

#[test]
fn check_random_swaps() {
    let mut prng = MersenneTwister::default();
    let mut builder = PermutationBuilder::new();
    for _ in 0..1000 {
        let length = prng.next_int64() % 100;
        let start_right = length + prng.next_int64() % (256 - length * 2 + 1);
        let start_left = prng.next_int64() % (start_right - length + 1);
        builder.swap_segment(start_left as u8, start_right as u8, length as u8);
    }
    let permutation = builder.build();
    assert!(permutations_are_inverses(&permutation));
}

#[test]
#[should_panic(expected = "assertion failed")]
fn sets_can_break_permutation() {
    let mut builder = PermutationBuilder::new();
    builder.set(7, 8);
    builder.build();
}

fn permutations_are_inverses(permutation: &Permutation) -> bool {
    (0u8..=255u8).all(|value| {
        permutation.backward(permutation.forward(value)) == value &&
            permutation.forward(permutation.backward(value)) == value
    })
}
