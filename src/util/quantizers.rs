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

pub struct OccurrenceCountQuantizer;

impl OccurrenceCountQuantizer {
    const SIGNIFICAND_BITS: usize = 1;

    pub fn max_output() -> usize {
        Self::quantize(<u16>::max_value())
    }

    pub fn quantize(input: u16) -> usize {
        if input < (1 << Self::SIGNIFICAND_BITS) {
            input as usize
        } else {
            let bit_length = (16 - input.leading_zeros()) as usize;
            let bits_cut = bit_length - Self::SIGNIFICAND_BITS - 1;
            let input = input as usize >> bits_cut;
            (bits_cut << Self::SIGNIFICAND_BITS) + input
        }
    }
}
