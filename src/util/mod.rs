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
pub mod hash;
pub mod indexer;
pub mod last_bytes;
pub mod permutation;
pub mod quantizers;

pub fn drain_full_option<T: Copy>(option: &mut Option<T>) -> T {
    assert!(option.is_some());
    let value = option.unwrap();
    *option = None;
    value
}

pub fn fill_empty_option<T>(option: &mut Option<T>, value: T) {
    assert!(option.is_none());
    *option = Some(value);
}

pub fn interpolate_f64(start: f64, stop: f64, intervals: u32)
                       -> impl Iterator<Item=f64> {
    (0..=intervals).map(move |step|
        stop * (step as f64 / intervals as f64) +
            start * ((intervals - step) as f64 / intervals as f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_f64_is_correct() {
        assert_eq!(interpolate_f64(1.0, 2.0, 4).collect::<Vec<_>>(),
                   vec![1.0, 1.25, 1.5, 1.75, 2.0]);
    }
}
