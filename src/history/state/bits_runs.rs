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
use bit::Bit;

#[derive(Clone, Debug, Copy, Eq, PartialEq)]
pub struct BitsRunsTracker(u32);

impl BitsRunsTracker {
    pub const NEW: Self = BitsRunsTracker(0);

    pub const MAX_RUN_LENGTH: u16 = 1000;

    const RUN_LENGTH_MASK: u32 = (1u32 << 10) - 1;

    pub fn new() -> Self { Self::NEW }

    pub fn for_bit_run(bit: Bit, run_length: u16) -> Self {
        let run_length = run_length.min(Self::MAX_RUN_LENGTH);
        BitsRunsTracker(bit.to_u32() + ((run_length as u32) << 1))
    }

    pub fn for_new_node(last_bit: Bit, opposite_bits_run_length: u16) -> Self {
        Self::for_bit_run(!last_bit, opposite_bits_run_length).updated(last_bit)
    }

    pub fn is_single_bit_run(&self) -> bool {
        (self.0 >> 1) <= (Self::MAX_RUN_LENGTH as u32)
    }

    pub fn last_bit(&self) -> Bit {
        ((self.0 & 1) == 1).into()
    }

    pub fn last_bit_run_length(&self) -> u16 {
        ((self.0 >> 1) & Self::RUN_LENGTH_MASK) as u16
    }

    pub fn opposite_bit_run_length(&self) -> u16 {
        ((self.0 >> 11) & Self::RUN_LENGTH_MASK) as u16
    }

    pub fn last_bit_previous_run_length(&self) -> u16 {
        ((self.0 >> 21) & Self::RUN_LENGTH_MASK) as u16
    }

    pub fn updated(&self, next_bit: Bit) -> Self {
        if next_bit == self.last_bit() {
            if self.last_bit_run_length() < Self::MAX_RUN_LENGTH {
                BitsRunsTracker(self.0 + (1u32 << 1))
            } else {
                BitsRunsTracker(self.0)
            }
        } else {
            BitsRunsTracker(next_bit.to_u32() | (1u32 << 1) |
                ((self.last_bit_run_length() as u32) << 11) |
                ((self.opposite_bit_run_length() as u32) << 21))
        }
    }
}
