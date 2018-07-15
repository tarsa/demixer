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
use fixed_point::{FixedPoint, fix_u16};
use fixed_point::types::Log2D;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CostTracker(u16);

impl CostTracker {
    pub const DECAY_SCALE: u8 = 4;
    const FRACTIONAL_BITS: u8 = Log2D::FRACTIONAL_BITS;

    pub const INITIAL: Self =
        CostTracker(1u16 << (Self::DECAY_SCALE + Self::FRACTIONAL_BITS));

    pub fn new(raw_value: u16) -> Self {
        CostTracker(raw_value)
    }

    pub fn raw(&self) -> u16 { self.0 }

    pub fn updated(&self, new_cost: Log2D) -> Self {
        let decayed = self.0 - fix_u16::scaled_down(self.0, Self::DECAY_SCALE);
        let new_cost = new_cost.raw();
        assert!(new_cost > 0);
        assert_eq!(new_cost as u16 as i32, new_cost);
        let new_cost = new_cost as u16;
        let result = decayed.saturating_add(new_cost);
        CostTracker(result)
    }
}
