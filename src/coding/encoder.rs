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
use std::io::Write;

use bit::Bit;
use fixed_point::FixedPoint;

use super::FinalProbability;

pub struct Encoder<'a> {
    writer: &'a mut Write,
    rc_buffer: i32,
    rc_range: i32,
    run_length_0xff: i32,
    last_output_byte: u8,
    delay: bool,
    carry: bool,
}

impl<'a> Encoder<'a> {
    pub fn new(writer: &'a mut Write) -> Encoder<'a> {
        Encoder {
            writer,
            rc_buffer: 0,
            rc_range: 0x7fff_ffff,
            run_length_0xff: 0,
            last_output_byte: 0,
            delay: false,
            carry: true,
        }
    }

    pub fn encode_bit(&mut self, probability: FinalProbability, value: Bit) {
        self.normalize();
        let rc_helper = ((self.rc_range as i64 * probability.raw() as i64)
            >> FinalProbability::FRACTIONAL_BITS) as i32;
        if value.is_1() {
            self.add_with_carry(rc_helper);
            self.rc_range -= rc_helper;
        } else {
            self.rc_range = rc_helper;
        }
    }

    pub fn encode_rare_event(&mut self, is_rare_event: bool) {
        self.normalize();
        if is_rare_event {
            let freq = self.rc_range - 1;
            self.add_with_carry(freq);
            self.rc_range = 1;
        } else {
            self.rc_range -= 1;
        }
    }

    fn normalize(&mut self) {
        assert_ne!(self.rc_range, 0);
        while self.rc_range < 0x0080_0000 {
            let shifted_byte = self.rc_buffer >> 23;
            self.output_byte(shifted_byte);
            self.rc_buffer = (self.rc_buffer << 8) & 0x7fff_ffff;
            self.rc_range <<= 8;
        }
    }

    fn output_byte(&mut self, octet: i32) {
        assert!(octet >= 0 && octet <= 255);
        let octet = octet as u8;
        if octet != 0xff || self.carry {
            if self.delay {
                let to_write = self.last_output_byte +
                    if self.carry { 1 } else { 0 };
                self.writer.write_all(&[to_write]).unwrap();
            }
            while self.run_length_0xff > 0 {
                self.run_length_0xff -= 1;
                let to_write = if self.carry { 0x00 } else { 0xff };
                self.writer.write_all(&[to_write]).unwrap();
            }
            self.last_output_byte = octet;
            self.delay = true;
            self.carry = false;
        } else {
            self.run_length_0xff += 1;
        }
    }

    fn add_with_carry(&mut self, cumulative_exclusive_fraction: i32) {
        self.rc_buffer =
            self.rc_buffer.wrapping_add(cumulative_exclusive_fraction);
        if self.rc_buffer < 0 {
            self.carry = true;
            self.rc_buffer &= 0x7fff_ffff;
        }
    }
}

impl<'a> Drop for Encoder<'a> {
    fn drop(&mut self) {
        for _ in 0..5 {
            let shifted_byte = (self.rc_buffer >> 23) & 0xff;
            self.output_byte(shifted_byte);
            self.rc_buffer <<= 8;
        }
    }
}
