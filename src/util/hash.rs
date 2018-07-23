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
pub struct Fnv1A(u64);

impl Fnv1A {
    pub fn new() -> Self {
        Fnv1A(0xcbf29ce484222325)
    }

    pub fn mix_byte(&mut self, input_byte: u8) {
        self.0 ^= input_byte as u64;
        self.0 = self.0.wrapping_mul(0x100000001b3);
    }

    pub fn into_u64(self) -> u64 {
        self.0
    }

    pub fn into_u32(self) -> u32 {
        let raw = self.0;
        (raw ^ (raw >> 32)) as u32
    }

    pub fn into_u16(self) -> u16 {
        let raw = self.into_u32();
        (raw ^ (raw >> 16)) as u16
    }
}
