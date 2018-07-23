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
use coding::FinalProbability;
use fixed_point::FixU32;
use fixed_point::types::{FractOnlyU32, StretchedProbD};
use lut::LookUpTables;
use mixing::apm::AdaptiveProbabilityMap;

pub struct PredictionFinalizer<'a> {
    luts: &'a LookUpTables,
    phase0_order0: AdaptiveProbabilityMap,
}

impl<'a> PredictionFinalizer<'a> {
    pub fn new(luts: &'a LookUpTables) -> Self {
        let make = |contexts, precision|
            AdaptiveProbabilityMap::new(contexts, precision, luts.squash_lut());
        PredictionFinalizer {
            luts,
            phase0_order0: make(256, 1),
        }
    }

    pub fn refine(&mut self, input_sq: FractOnlyU32, input_st: StretchedProbD,
                  current_byte: u8) -> FinalProbability {
        let refined_sq = self.phase0_order0.refine(
            current_byte as usize, input_sq, input_st, self.luts.apm_lut(1));
        refined_sq.to_fix_u32()
    }

    pub fn update(&mut self, input_bit: Bit, current_byte: u8) {
        self.phase0_order0.update_predictions(
            current_byte as usize, input_bit, 10, true);
    }
}
