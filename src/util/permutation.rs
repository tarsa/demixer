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
use std::collections::HashSet;

pub struct Permutation {
    forward: [u8; 256],
    backward: [u8; 256],
}

impl Permutation {
    pub fn forward(&self, original: u8) -> u8 {
        self.forward[original as usize]
    }

    pub fn backward(&self, permuted: u8) -> u8 {
        self.backward[permuted as usize]
    }
}

pub struct PermutationBuilder([u8; 256]);

impl PermutationBuilder {
    pub fn new() -> Self {
        let mut array = [0u8; 256];
        for i in 0..array.len() { array[i] = i as u8 };
        PermutationBuilder(array)
    }

    pub fn build(self) -> Permutation {
        let forward = self.0;
        assert_eq!(forward.iter().collect::<HashSet<_>>().len(), 256);
        let mut backward = [0u8; 256];
        for (index, &elem) in forward.iter().enumerate() {
            backward[elem as usize] = index as u8;
        }
        Permutation { forward, backward }
    }

    pub fn set(&mut self, from: u8, to: u8) -> &mut Self {
        self.0[from as usize] = to;
        self
    }

    pub fn swap_pair(&mut self, elem_1: u8, elem_2: u8) -> &mut Self {
        let value_1 = self.0[elem_1 as usize];
        let value_2 = self.0[elem_2 as usize];
        self.set(elem_1, value_2);
        self.set(elem_2, value_1);
        self
    }

    pub fn swap_segment(&mut self, start_left: u8, start_right: u8, length: u8)
                        -> &mut Self {
        assert!(start_left as usize + length as usize <= start_right as usize);
        assert!(start_right as usize + length as usize <= self.0.len());
        for index in 0..length {
            self.swap_pair(start_left + index, start_right + index);
        }
        self
    }
}
