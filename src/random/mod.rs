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
pub struct MersenneTwister {
    state: [u64; MersenneTwister::NN],
    index: usize,
}

impl MersenneTwister {
    const NN: usize = 312;
    const MM: usize = 156;
    const MATRIX_A: u64 = 0xB5026F5AA96619E9u64;
    const UM: u64 = 0xFFFFFFFF80000000u64; /* Most significant 33 bits */
    const LM: u64 = 0x7FFFFFFFu64; /* Least significant 31 bits */
    const MAG01: [u64; 2] = [0, Self::MATRIX_A];

    pub fn new_by_scalar_seed(seed: u64) -> Self {
        let mut state_vector = [0; MersenneTwister::NN];
        state_vector[0] = seed;
        for index in 1..Self::NN {
            state_vector[index] = 6364136223846793005u64.wrapping_mul(
                state_vector[index - 1] ^ (state_vector[index - 1] >> 62))
                .wrapping_add(index as u64);
        }
        MersenneTwister {
            state: state_vector,
            index: Self::NN,
        }
    }

    pub fn new_by_vector_seed(init_key: &[u64]) -> Self {
        let mut i = 1usize;
        let mut j = 0usize;
        let mut mt = Self::new_by_scalar_seed(19650218);
        for _ in 0..Self::NN.max(init_key.len()) {
            mt.state[i] = (mt.state[i] ^
                ((mt.state[i - 1] ^ (mt.state[i - 1] >> 62))
                    .wrapping_mul(3935559000370003845)))
                .wrapping_add(init_key[j])
                .wrapping_add(j as u64); /* non linear */
            i += 1;
            j += 1;
            if i >= Self::NN {
                mt.state[0] = mt.state[Self::NN - 1];
                i = 1;
            }
            if j >= init_key.len() {
                j = 0;
            };
        }
        for _ in 1..Self::NN {
            mt.state[i] = (mt.state[i] ^
                ((mt.state[i - 1] ^ (mt.state[i - 1] >> 62))
                    .wrapping_mul(2862933555777941757)))
                .wrapping_sub(i as u64); /* non linear */
            i += 1;
            if i >= Self::NN {
                mt.state[0] = mt.state[Self::NN - 1];
                i = 1;
            }
        }
        mt.state[0] = 1 << 63; /* MSB is 1; assuring non-zero initial array */
        mt
    }

    /** generates a random number on [0, 2^64-1]-interval */
    pub fn next_int64(&mut self) -> u64 {
        let mut x: u64;
        assert!(self.index <= Self::NN);
        /* generate NN words at one time */
        if self.index == Self::NN {
            for i in 0..Self::NN - Self::MM {
                x = (self.state[i] & Self::UM) | (self.state[i + 1] & Self::LM);
                self.state[i] = self.state[i + Self::MM] ^ (x >> 1)
                    ^ Self::MAG01[x as usize & 1];
            }
            for i in Self::NN - Self::MM..Self::NN - 1 {
                x = (self.state[i] & Self::UM) | (self.state[i + 1] & Self::LM);
                self.state[i] = self.state[i + Self::MM - Self::NN] ^ (x >> 1)
                    ^ Self::MAG01[x as usize & 1];
            }
            x = (self.state[Self::NN - 1] & Self::UM) |
                (self.state[0] & Self::LM);
            self.state[Self::NN - 1] = self.state[Self::MM - 1] ^ (x >> 1)
                ^ Self::MAG01[x as usize & 1];
            self.index = 0;
        }
        x = self.state[self.index];
        self.index += 1;

        x ^= (x >> 29) & 0x5555555555555555;
        x ^= (x << 17) & 0x71D67FFFEDA60000;
        x ^= (x << 37) & 0xFFF7EEE000000000;
        x ^= x >> 43;
        x
    }

    /** generates a random number on [0, 2^63-1]-interval */
    pub fn next_int63(&mut self) -> i64 {
        (self.next_int64() >> 1) as i64
    }

    /** generates a random number on [0,1]-real-interval */
    pub fn next_real1(&mut self) -> f64 {
        (self.next_int64() >> 11) as f64 * (1.0 / 9007199254740991.0)
    }

    /** generates a random number on [0,1)-real-interval */
    pub fn next_real2(&mut self) -> f64 {
        (self.next_int64() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }

    /** generates a random number on (0,1)-real-interval */
    pub fn next_real3(&mut self) -> f64 {
        ((self.next_int64() >> 12) as f64 + 0.5) * (1.0 / 4503599627370496.0)
    }
}

impl Default for MersenneTwister {
    fn default() -> Self {
        Self::new_by_scalar_seed(5489)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;
    use std::vec::Vec;

    #[test]
    fn output_matches_test_vector() {
        let mt_output: &'static str = include_str!("mt19937-64.out.txt");
        let test_vector =
            mt_output.lines().skip(1).take(200)
                .flat_map(|line| line.split_whitespace())
                .map(|as_str| u64::from_str(as_str).unwrap())
                .collect::<Vec<_>>();
        let init = [0x12345, 0x23456, 0x34567, 0x45678];
        let mut mt = MersenneTwister::new_by_vector_seed(&init);
        assert_eq!(test_vector.len(), 1000);
        for &item in test_vector.iter() {
            assert_eq!(item, mt.next_int64());
        }
    }
}
