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
use std::io::Read;

use bit::Bit;
use entropy::FinalProbability;
use fixed_point::FixedPoint;

pub struct Decoder<'a> {
    reader: &'a mut Read,
    rc_buffer: i32,
    rc_range: i32,
    next_high_bit: i32,
}

impl<'a> Decoder<'a> {
    pub fn new(reader: &'a mut Read) -> Decoder<'a> {
        let mut decoder =
            Decoder {
                reader,
                rc_buffer: 0,
                rc_range: 0x7fff_ffff,
                next_high_bit: 0,
            };
        for _ in 0..4 {
            decoder.rc_buffer <<= 8;
            decoder.rc_buffer += decoder.input_byte();
        }
        decoder
    }

    pub fn decode_bit(&mut self, probability: FinalProbability) -> Bit {
        self.normalize();
        let rc_helper = ((self.rc_range as i64 * probability.raw() as i64)
            >> FinalProbability::FRACTIONAL_BITS) as i32;
        if self.rc_buffer < rc_helper {
            self.rc_range = rc_helper;
            false.into()
        } else {
            self.rc_range -= rc_helper;
            self.rc_buffer -= rc_helper;
            true.into()
        }
    }

    pub fn decode_rare_event(&mut self) -> bool {
        self.normalize();
        if self.rc_buffer < self.rc_range - 1 {
            self.rc_range -= 1;
            false
        } else {
            self.rc_buffer = 0;
            self.rc_range = 1;
            true
        }
    }

    fn input_byte(&mut self) -> i32 {
        let mut input = [0u8];
        self.reader.read_exact(&mut input).unwrap();
        let input_byte = input[0] as i32;
        let current_byte = (input_byte >> 1) + (self.next_high_bit << 7);
        self.next_high_bit = input_byte & 1;
        current_byte
    }

    fn normalize(&mut self) {
        while self.rc_range < 0x0080_0000 {
            self.rc_buffer <<= 8;
            self.rc_buffer += self.input_byte();
            self.rc_range <<= 8;
        }
    }
}
