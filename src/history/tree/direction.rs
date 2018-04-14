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
use std::ops;

use bit::Bit;
use super::node_child::NodeChild;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Left = 0,
    Right = 1,
}

impl Direction {
    pub fn fold<T, FL: FnOnce() -> T, FR: FnOnce() -> T>(
        &self, on_left: FL, on_right: FR) -> T {
        match *self {
            Direction::Left => on_left(),
            Direction::Right => on_right(),
        }
    }
}

impl ops::Index<Direction> for [NodeChild; 2] {
    type Output = NodeChild;

    fn index(&self, index: Direction) -> &NodeChild {
        match index {
            Direction::Left => &self[0],
            Direction::Right => &self[1],
        }
    }
}

impl ops::IndexMut<Direction> for [NodeChild; 2] {
    fn index_mut(&mut self, index: Direction) -> &mut NodeChild {
        match index {
            Direction::Left => &mut self[0],
            Direction::Right => &mut self[1],
        }
    }
}

impl From<Bit> for Direction {
    fn from(value: Bit) -> Direction {
        match value {
            Bit::Zero => Direction::Left,
            Bit::One => Direction::Right,
        }
    }
}

impl ops::Not for Direction {
    type Output = Direction;

    fn not(self) -> Direction {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}
