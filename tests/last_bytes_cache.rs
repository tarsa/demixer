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

use demixer::bit::Bit;
use demixer::random::MersenneTwister;
use demixer::util::last_bytes::{UnfinishedByte, LastBytesCache};

#[test]
fn sanity_checks() {
    let mut cache = LastBytesCache::new();
    cache.start_new_byte();
    assert_eq!(cache.unfinished_byte(), UnfinishedByte::EMPTY);
}

#[test]
fn test_vector() {
    let mut cache = LastBytesCache::new();
    cache.start_new_byte();
    let input_bytes = ['b' as u8, 'a' as u8, 'n' as u8, 'g' as u8];
    for input_byte in input_bytes.iter() {
        for bit_index in (0..=7).rev() {
            let input_bit: Bit = ((input_byte & (1 << bit_index)) != 0).into();
            cache.on_next_bit(input_bit);
        }
        cache.start_new_byte();
    }
    assert_eq!(cache.unfinished_byte(), UnfinishedByte::EMPTY);
    assert_eq!(cache.previous_byte_1(), input_bytes[3]);
    assert_eq!(cache.previous_byte_2(), input_bytes[2]);
    assert_eq!(cache.previous_byte_3(), input_bytes[1]);
}

#[test]
#[should_panic(expected = "bit_index >= 0")]
fn starting_new_byte_required_on_start() {
    let cache = LastBytesCache::new();
    cache.unfinished_byte();
}

#[test]
#[should_panic(expected = "bit_index >= 0")]
fn starting_new_byte_required_every_8_bits() {
    let mut cache = LastBytesCache::new();
    cache.start_new_byte();
    for _ in 0..8 { cache.on_next_bit(Bit::Zero); }
    cache.unfinished_byte();
}

#[test]
fn current_and_previous_bytes_are_properly_reported() {
    let mut prng = MersenneTwister::default();
    let mut cache = LastBytesCache::new();
    let mut previous_byte_3 = 0u8;
    let mut previous_byte_2 = 0u8;
    let mut previous_byte_1 = 0u8;
    for _ in 0..123456 {
        let input_byte = prng.next_int64() as u8;
        cache.start_new_byte();
        assert_eq!(cache.previous_byte_3(), previous_byte_3);
        assert_eq!(cache.previous_byte_2(), previous_byte_2);
        assert_eq!(cache.previous_byte_1(), previous_byte_1);
        let mut unfinished_byte = 1u8;
        for bit_index in (0..=7).rev() {
            assert_eq!(cache.unfinished_byte().raw(), unfinished_byte);
            let input_bit: Bit = ((input_byte >> bit_index) & 1 == 1).into();
            cache.on_next_bit(input_bit);
            unfinished_byte <<= 1;
            unfinished_byte += input_bit.to_u8();
        }
        previous_byte_3 = previous_byte_2;
        previous_byte_2 = previous_byte_1;
        previous_byte_1 = input_byte;
    }
}

#[test]
fn hash01_has_no_collisions() {
    let mut occurrence_array = [false; 256 * 256];
    for bit_length in 8..=15 {
        for input in 0..1u16 << bit_length {
            let mut cache = LastBytesCache::new();
            for bit_index in (0..bit_length).rev() {
                if (bit_length - 1 - bit_index) % 8 == 0 {
                    cache.start_new_byte();
                }
                let input_bit: Bit = (((input >> bit_index) & 1) == 1).into();
                cache.on_next_bit(input_bit);
            }
            if bit_length % 8 == 0 {
                cache.start_new_byte();
            }
            let hash = cache.hash01_16() as usize;
            assert!(!occurrence_array[hash]);
            occurrence_array[hash] = true;
        }
    }
    for index in 0..256 { assert!(!occurrence_array[index]); }
    for index in 256..256 * 256 { assert!(occurrence_array[index]); }
}

#[test]
fn all_hashes_use_previous_byte_1() {
    let mut prng = MersenneTwister::default();
    for _ in 0..12345 {
        let seeded_cache = seed_cache(&mut prng);
        for &(bytes_1, bytes_2) in [
            (any_length(&[0x00]), any_length(&[0x01])),
            (&[0xb5, 0x00], &[0xb5, 0x01]),
            (&[0xff, 0xb5, 0x00], &[0xff, 0xb5, 0x01]),
        ].iter() {
            assert!(!hash_equal_after_push(&seeded_cache, |h| h.hash01_16(),
                                           bytes_1, bytes_2));
            assert!(!hash_equal_after_push(&seeded_cache, |h| h.hash02_16(),
                                           bytes_1, bytes_2));
            assert!(!hash_equal_after_push(&seeded_cache, |h| h.hash03_16(),
                                           bytes_1, bytes_2));
        }
    }
}

#[test]
fn two_hashes_use_previous_byte_2() {
    let mut prng = MersenneTwister::default();
    for _ in 0..12345 {
        let seeded_cache = seed_cache(&mut prng);
        for &(bytes_1, bytes_2) in [
            (any_length(&[0x00, 0x34]), any_length(&[0x01, 0x34])),
            (&[0xb5, 0x00, 0x34], &[0xb5, 0x01, 0x34]),
            (&[0xff, 0xb5, 0x00, 0x34], &[0xff, 0xb5, 0x01, 0x34]),
        ].iter() {
            assert!(hash_equal_after_push(&seeded_cache, |h| h.hash01_16(),
                                          bytes_1, bytes_2));
            assert!(!hash_equal_after_push(&seeded_cache, |h| h.hash02_16(),
                                           bytes_1, bytes_2));
            assert!(!hash_equal_after_push(&seeded_cache, |h| h.hash03_16(),
                                           bytes_1, bytes_2));
        }
    }
}

#[test]
fn one_hash_uses_previous_byte_3() {
    let mut prng = MersenneTwister::default();
    for _ in 0..12345 {
        let seeded_cache = seed_cache(&mut prng);
        for &(bytes_1, bytes_2) in [
            (any_length(&[0x00, 0x67, 0x34]), any_length(&[0x01, 0x67, 0x34])),
            (&[0xb5, 0x00, 0x67, 0x34], &[0xb5, 0x01, 0x67, 0x34]),
            (&[0xff, 0xb3, 0x01, 0x34], &[0xff, 0xb5, 0x01, 0x34]),
        ].iter() {
            assert!(hash_equal_after_push(&seeded_cache, |h| h.hash01_16(),
                                          bytes_1, bytes_2));
            assert!(hash_equal_after_push(&seeded_cache, |h| h.hash02_16(),
                                          bytes_1, bytes_2));
            assert!(!hash_equal_after_push(&seeded_cache, |h| h.hash03_16(),
                                           bytes_1, bytes_2));
        }
    }
}

fn seed_cache(prng: &mut MersenneTwister) -> LastBytesCache {
    let mut input = prng.next_int64();
    let mut cache = LastBytesCache::new();
    for _ in 0..8 {
        cache.start_new_byte();
        for _ in 0..8 {
            let input_bit: Bit = ((input & 1) == 1).into();
            input >>= 1;
            cache.on_next_bit(input_bit);
        }
    }
    cache.start_new_byte();
    cache
}

fn any_length<T>(slice: &[T]) -> &[T] {
    slice
}

fn hash_equal_after_push<F>(cache: &LastBytesCache, hash: F,
                            bytes_1: &[u8], bytes_2: &[u8]) -> bool
    where F: Fn(&LastBytesCache) -> u16 {
    let mut cache_1 = cache.clone();
    bytes_1.iter().for_each(|&byte_1| push_byte(&mut cache_1, byte_1));
    let mut cache_2 = cache.clone();
    bytes_2.iter().for_each(|&byte_2| push_byte(&mut cache_2, byte_2));
    hash(&cache_1) == hash(&cache_2)
}

fn push_byte(cache: &mut LastBytesCache, input_byte: u8) {
    for bit_index in (0..=7).rev() {
        let input_bit: Bit = ((input_byte >> bit_index) & 1 == 1).into();
        cache.on_next_bit(input_bit);
    }
    cache.start_new_byte();
}
