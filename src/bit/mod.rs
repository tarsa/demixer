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
use std::fmt;
use std::ops::Not;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Bit { Zero = 0, One = 1 }

impl Bit {
    pub fn is_0(&self) -> bool { *self == Bit::Zero }
    pub fn is_1(&self) -> bool { *self == Bit::One }

    pub fn to_i32(&self) -> i32 { *self as i32 }
    pub fn to_u32(&self) -> u32 { *self as u32 }
    pub fn to_i64(&self) -> i64 { *self as i64 }
    pub fn to_u64(&self) -> u64 { *self as u64 }
}

impl fmt::Display for Bit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_i32())
    }
}

impl From<bool> for Bit {
    fn from(value: bool) -> Self {
        if value { Bit::One } else { Bit::Zero }
    }
}

impl Not for Bit {
    type Output = Bit;

    fn not(self) -> Bit {
        match self {
            Bit::Zero => Bit::One,
            Bit::One => Bit::Zero,
        }
    }
}
