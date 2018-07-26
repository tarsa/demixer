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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RecentBitsHistory(u16);

impl RecentBitsHistory {
    pub const INVALID: Self = RecentBitsHistory(0);

    const MAX_LENGTH: u8 = 15;

    const MASK: u16 = (1u16 << Self::MAX_LENGTH) - 1 + (1 << Self::MAX_LENGTH);

    pub fn new_unchecked(raw_state: u16) -> Self {
        RecentBitsHistory(raw_state)
    }

    pub fn for_bit_run(repeating_bit: Bit, run_length: u16) -> Self {
        let run_length =
            run_length.min(RecentBitsHistory::MAX_LENGTH as u16) as u8;
        let bit = repeating_bit.to_u16();
        let leading_bit = 1u16 << run_length;
        let history_bits = (bit << run_length) - bit + (bit << run_length);
        RecentBitsHistory(leading_bit | history_bits)
    }

    pub fn for_new_node(last_bit: Bit, opposite_bits_run_length: u16) -> Self {
        Self::for_bit_run(!last_bit, opposite_bits_run_length).updated(last_bit)
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    pub fn histogram(&self) -> u8 {
        assert_ne!(self.0, 0);
        let history_length = Self::MAX_LENGTH - self.0.leading_zeros() as u8;
        let ones_count = (self.0.count_ones() - 1) as u8;
        let zeros_count = history_length - ones_count;
        assert!(ones_count <= 15 && zeros_count <= 15);
        (ones_count << 4) | zeros_count
    }

    pub fn last_7_bits(&self) -> u8 {
        let capped = self.0 & 127;
        let leading_bit = if self.0 >= 128 { 128 } else { 0 };
        (leading_bit | capped) as u8
    }

    pub fn updated(&self, next_bit: Bit) -> Self {
        let highest_leading_bit = self.0 & (1 << Self::MAX_LENGTH);
        let shifted = ((self.0 << 1) + next_bit.to_u16()) & Self::MASK;
        RecentBitsHistory(shifted | highest_leading_bit)
    }
}
