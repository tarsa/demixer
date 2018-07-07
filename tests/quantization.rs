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
extern crate demixer;

use demixer::util::quantizers::OccurrenceCountQuantizer;

#[test]
fn occurrence_count_quantizer_is_monotonic() {
    for input in 0..<u16>::max_value() {
        let current_output = OccurrenceCountQuantizer::quantize(input);
        let next_output = OccurrenceCountQuantizer::quantize(input);
        assert!(next_output - current_output <= 1, "input: {}", input);
    }
}
