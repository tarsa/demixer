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
use DO_CHECKS;
use bit::Bit;
use super::hash::Fnv1A;

#[derive(Debug, Eq, PartialEq)]
pub struct UnfinishedByte(u8);

impl UnfinishedByte {
    pub const EMPTY: Self = UnfinishedByte(1);

    pub fn raw(&self) -> u8 { self.0 }
}

#[derive(Clone)]
pub struct LastBytesCache {
    bit_index: i32,
    bytes_cache: u32,
}

impl LastBytesCache {
    pub fn new() -> Self {
        LastBytesCache {
            bit_index: -1,
            bytes_cache: 0,
        }
    }

    pub fn start_new_byte(&mut self) {
        assert_eq!(self.bit_index, -1);
        self.bit_index = 7;
        self.bytes_cache <<= 8;
    }

    pub fn on_next_bit(&mut self, input_bit: Bit) {
        let bit_index = self.checked_bit_index();
        self.bytes_cache |= input_bit.to_u32() << bit_index;
        self.bit_index -= 1;
    }

    pub fn unfinished_byte(&self) -> UnfinishedByte {
        let bit_index = self.checked_bit_index();
        let raw = (1u8 << (7 - bit_index)) +
            ((self.bytes_cache & 0xff) >> (bit_index + 1)) as u8;
        UnfinishedByte(raw)
    }

    pub fn previous_byte_1(&self) -> u8 {
        if DO_CHECKS { assert!(self.bit_index >= 0); }
        (self.bytes_cache >> 8) as u8
    }

    pub fn previous_byte_2(&self) -> u8 {
        if DO_CHECKS { assert!(self.bit_index >= 0); }
        (self.bytes_cache >> 16) as u8
    }

    pub fn previous_byte_3(&self) -> u8 {
        if DO_CHECKS { assert!(self.bit_index >= 0); }
        (self.bytes_cache >> 24) as u8
    }

    pub fn hash01_16(&self) -> u16 {
        let bit_index = self.checked_bit_index();
        (1u16 << (8 + 7 - bit_index)) +
            ((self.bytes_cache & 0xff_ff) >> (bit_index + 1)) as u16
    }

    pub fn hash02_16(&self) -> u16 {
        if DO_CHECKS { assert!(self.bit_index >= 0); }
        let mut hasher = Fnv1A::new();
        hasher.mix_byte(self.previous_byte_2());
        hasher.mix_byte(self.previous_byte_1());
        hasher.mix_byte(self.unfinished_byte().raw());
        hasher.into_u16()
    }

    pub fn hash03_16(&self) -> u16 {
        if DO_CHECKS { assert!(self.bit_index >= 0); }
        let mut hasher = Fnv1A::new();
        hasher.mix_byte(self.previous_byte_3());
        hasher.mix_byte(self.previous_byte_2());
        hasher.mix_byte(self.previous_byte_1());
        hasher.mix_byte(self.unfinished_byte().raw());
        hasher.into_u16()
    }

    fn checked_bit_index(&self) -> usize {
        assert!(self.bit_index >= 0);
        self.bit_index as usize
    }
}
