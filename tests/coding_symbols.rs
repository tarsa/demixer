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

use std::io::{Read, Write};

use demixer::bit::Bit;
use demixer::coding::FinalProbability;
use demixer::coding::decoder::Decoder;
use demixer::coding::encoder::Encoder;
use demixer::fixed_point::*;
use demixer::fixed_point::types::Log2Q;
use demixer::lut::log2::Log2Lut;
use demixer::random::MersenneTwister;

enum CodingEvent {
    BitWithProb { probability: FinalProbability, value: Bit },
    RareEvent { happened: bool },
}

#[test]
fn coding_is_reversible_for_empty_input() {
    check_coding_is_reversible(&[], Some(4));
}

#[test]
fn coding_is_reversible_for_single_event() {
    check_coding_is_reversible(&[bit_with_prob(0x73_4396, true)], Some(4));
    check_coding_is_reversible(&[bit_with_prob(0x73_4396, false)], Some(4));
    check_coding_is_reversible(&[rare_event(true)], Some(4));
    check_coding_is_reversible(&[rare_event(false)], Some(4));
}

#[test]
fn coding_is_reversible_for_multiple_events() {
    let mut prng = MersenneTwister::new_by_scalar_seed(1500100900);
    for &length in [10, 20, 30, 80, 200, 1000, 8000].iter() {
        let events = generate_events(&mut prng, length, false);
        check_coding_is_reversible(&events, None);
    }
}

#[test]
fn coding_can_be_estimated() {
    let mut prng = MersenneTwister::new_by_scalar_seed(1500100900);
    let lut = Log2Lut::new();
    for &length in [0, 1, 2, 3, 4, 5, 10, 20, 30, 80, 200, 1000, 8000].iter() {
        for _ in 0..5 {
            let events = generate_events(&mut prng, length, true);
            let mut size = Log2Q::new_unchecked(0);
            for event in events.iter() {
                let event_cost = estimate_event_cost(event, &lut);
                size = size.add(&event_cost);
            }
            let mut buffer = Vec::new();
            encode_events(&events, &mut buffer);
            assert!(buffer.len() * 8 - (size.trunc() as usize) <= 32,
                    "{} {}", buffer.len() * 8, size.trunc() as usize);
        }
    }
}

fn bit_with_prob(probability: u32, value: bool) -> CodingEvent {
    CodingEvent::BitWithProb {
        probability: FinalProbability::new(probability, 23),
        value: value.into(),
    }
}

fn rare_event(happened: bool) -> CodingEvent {
    CodingEvent::RareEvent { happened }
}

fn generate_events(prng: &mut MersenneTwister, length: usize,
                   only_bit_with_prob: bool) -> Vec<CodingEvent> {
    let mut events = Vec::new();
    for _ in 0..length {
        if only_bit_with_prob || prng.next_real1() > 0.1 {
            let bit = prng.next_real1() < 0.5;
            let prob_raw = prng.next_int64() &
                ((1 << FinalProbability::FRACTIONAL_BITS) - 1);
            let prob_raw = if prob_raw == 0 { 1 } else { prob_raw };
            events.push(bit_with_prob(prob_raw as u32, bit))
        } else {
            let happened = prng.next_real1() < 0.01;
            events.push(rare_event(happened))
        }
    }
    events
}

fn check_coding_is_reversible(events: &[CodingEvent],
                              expected_length_opt: Option<usize>) {
    let mut buffer = Vec::new();

    encode_events(events, &mut buffer);
    expected_length_opt.into_iter().for_each(|expected_length|
        assert_eq!(buffer.len(), expected_length));

    let reader: &mut Read = &mut buffer.as_slice();
    decode_events(events, reader);
    assert_eq!(reader.bytes().count(), 0);
}

fn encode_events(events: &[CodingEvent], writer: &mut Write) {
    let mut encoder = Encoder::new(writer);
    for event in events {
        match event {
            &CodingEvent::BitWithProb { ref probability, value } =>
                encoder.encode_bit(probability.clone(), value),
            &CodingEvent::RareEvent { happened } =>
                encoder.encode_rare_event(happened),
        }
    }
}

fn decode_events(events: &[CodingEvent], reader: &mut Read) {
    let mut decoder = Decoder::new(reader);
    for event in events {
        match event {
            &CodingEvent::BitWithProb { ref probability, value } =>
                assert_eq!(decoder.decode_bit(probability.clone()), value),
            &CodingEvent::RareEvent { happened } =>
                assert_eq!(decoder.decode_rare_event(), happened),
        }
    }
}

fn estimate_event_cost(event: &CodingEvent, lut: &Log2Lut) -> Log2Q {
    match event {
        &CodingEvent::BitWithProb { ref probability, value } =>
            probability.estimate_cost(value, lut).to_fix_i64::<Log2Q>(),
        &CodingEvent::RareEvent { .. } =>
            panic!("rare event can't be estimated"),
    }
}
