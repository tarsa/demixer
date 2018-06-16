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
pub mod recent_bits;

use bit::Bit;

// TODO replace that with generics
pub type TheHistoryState = self::recent_bits::RecentBitsState;
pub type TheHistoryStateFactory = self::recent_bits::RecentBitsStateFactory;

pub trait HistoryState: Sized + Copy + Clone + Eq {
    const INVALID: Self;

    fn is_valid(&self) -> bool;

    fn compressed_state(&self) -> u8;

    /** Last few bits with leading 1 */
    fn last_bits(&self) -> u8;

    fn updated(&self, next_bit: Bit) -> Self;
}

pub trait HistoryStateFactory {
    type HistoryType: HistoryState;

    fn new() -> Self;

    fn for_bit_run(&self, repeating_bit: Bit,
                   run_length: u16) -> Self::HistoryType;

    fn for_new_node(&self, last_bit: Bit,
                    opposite_bits_run_length: u16) -> Self::HistoryType;
}
