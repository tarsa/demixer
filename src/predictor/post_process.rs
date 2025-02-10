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
use fixed_point::{FixedPoint, FixU32, fix_u64};
use fixed_point::types::{FractOnlyU32, StretchedProbD};
use lut::LookUpTables;
use mixing::apm::AdaptiveProbabilityMap;
use mixing::mixer::{MixerInitializationMode, Mixer, MixerN};
use util::{drain_full_option, fill_empty_option};
use util::last_bytes::LastBytesCache;

#[allow(dead_code)]
enum Mode { None, Light, Adaptive }

const MODE: Mode = Mode::Adaptive;

pub struct PredictionFinalizer<'a> {
    luts: &'a LookUpTables,
    phase0_order0: AdaptiveProbabilityMap,
    phase1_order1: AdaptiveProbabilityMap,
    phase1_order2: AdaptiveProbabilityMap,
    phase1_order3: AdaptiveProbabilityMap,
    mixers: Vec<[MixerN; 4]>,
    mixers_row_index_opt: Option<usize>,
    mixing_result_opt: Option<(FractOnlyU32, StretchedProbD)>,
}

impl<'a> PredictionFinalizer<'a> {
    const INDEX_SCALE_DOWN_BITS: u8 = 1;

    const FACTORS: [u8; 13] = [4, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9];

    pub fn new(luts: &'a LookUpTables) -> Self {
        let make = |contexts, precision|
            AdaptiveProbabilityMap::new(contexts, precision, luts.squash_lut());
        let make_mixers_row = || [
            MixerN::new(2, 100, MixerInitializationMode::EqualSummingToOne),
            MixerN::new(3, 100, MixerInitializationMode::EqualSummingToOne),
            MixerN::new(4, 100, MixerInitializationMode::EqualSummingToOne),
            MixerN::new(5, 100, MixerInitializationMode::EqualSummingToOne),
        ];
        let mut mixers = Vec::with_capacity(
            StretchedProbD::intervals_count(Self::INDEX_SCALE_DOWN_BITS));
        for _ in 0..mixers.capacity() {
            mixers.push(make_mixers_row());
        }
        match MODE {
            Mode::None =>
                PredictionFinalizer {
                    luts,
                    phase0_order0: make(0, 0),
                    phase1_order1: make(0, 0),
                    phase1_order2: make(0, 0),
                    phase1_order3: make(0, 0),
                    mixers: Vec::new(),
                    mixers_row_index_opt: None,
                    mixing_result_opt: None,
                },
            Mode::Light =>
                PredictionFinalizer {
                    luts,
                    phase0_order0: make(256, 0),
                    phase1_order1: make(256 * 256, 0),
                    phase1_order2: make(256 * 256, 0),
                    phase1_order3: make(256 * 256, 0),
                    mixers: Vec::new(),
                    mixers_row_index_opt: None,
                    mixing_result_opt: None,
                },
            Mode::Adaptive =>
                PredictionFinalizer {
                    luts,
                    phase0_order0: make(256, 0),
                    phase1_order1: make(256 * 256, Self::INDEX_SCALE_DOWN_BITS),
                    phase1_order2: make(256 * 256, Self::INDEX_SCALE_DOWN_BITS),
                    phase1_order3: make(256 * 256, Self::INDEX_SCALE_DOWN_BITS),
                    mixers,
                    mixers_row_index_opt: None,
                    mixing_result_opt: None,
                },
        }
    }

    pub fn refine(&mut self, input_sq: FractOnlyU32, input_st: StretchedProbD,
                  contexts_count: usize, last_bytes: &LastBytesCache)
                  -> FinalProbability {
        assert_eq!(self.mixing_result_opt, None);
        let stretch_lut = self.luts.stretch_lut();
        match MODE {
            Mode::None =>
                input_sq.to_fix_u32(),
            Mode::Light => {
                let p0_o0_sq = self.phase0_order0.refine(
                    last_bytes.unfinished_byte().raw() as usize,
                    input_sq, input_st, self.luts.apm_lut(0));
                let p0_mix_sq_raw = fix_u64::scaled_down(
                    input_sq.raw() as u64 * 3 + p0_o0_sq.raw() as u64, 2);
                let p0_mix_sq = FractOnlyU32::new(p0_mix_sq_raw as u32, 31);
                let p0_mix_st = stretch_lut.stretch(p0_mix_sq);
                let p1_o1_sq = self.phase1_order1.refine(
                    last_bytes.hash01_16() as usize, p0_mix_sq, p0_mix_st,
                    self.luts.apm_lut(0));
                let p1_o2_sq = self.phase1_order2.refine(
                    last_bytes.hash02_16() as usize, p0_mix_sq, p0_mix_st,
                    self.luts.apm_lut(0));
                let p1_o3_sq = self.phase1_order3.refine(
                    last_bytes.hash03_16() as usize, p0_mix_sq, p0_mix_st,
                    self.luts.apm_lut(0));
                let output_sq_raw = fix_u64::scaled_down(
                    p1_o1_sq.raw() as u64 + p1_o2_sq.raw() as u64 * 2 +
                        p1_o3_sq.raw() as u64, 2);
                let output_sq = FractOnlyU32::new(output_sq_raw as u32, 31);
                output_sq.to_fix_u32()
            }
            Mode::Adaptive => {
                let mixers_row_index = input_st.to_interval_index(
                    Self::INDEX_SCALE_DOWN_BITS);
                fill_empty_option(&mut self.mixers_row_index_opt,
                                  mixers_row_index);
                let mixer_index = quantize_contexts_count(contexts_count);
                let mixer = &mut self.mixers[mixers_row_index][mixer_index];
                let p0_o0_sq = self.phase0_order0.refine(
                    last_bytes.unfinished_byte().raw() as usize,
                    input_sq, input_st, self.luts.apm_lut(0));
                let p0_mix_sq_raw = fix_u64::scaled_down(
                    input_sq.raw() as u64 * 3 + p0_o0_sq.raw() as u64, 2);
                let p0_mix_sq = FractOnlyU32::new(p0_mix_sq_raw as u32, 31);
                let p0_mix_st = stretch_lut.stretch(p0_mix_sq);
                mixer.set_input(0, input_sq, input_st);
                mixer.set_input(1, p0_o0_sq, stretch_lut.stretch(p0_o0_sq));
                if mixer_index >= 1 {
                    let p1_o1_sq = self.phase1_order1.refine(
                        last_bytes.hash01_16() as usize, p0_mix_sq, p0_mix_st,
                        self.luts.apm_lut(Self::INDEX_SCALE_DOWN_BITS));
                    mixer.set_input(2, p1_o1_sq, stretch_lut.stretch(p1_o1_sq));
                }
                if mixer_index >= 2 {
                    let p1_o2_sq = self.phase1_order2.refine(
                        last_bytes.hash02_16() as usize, p0_mix_sq, p0_mix_st,
                        self.luts.apm_lut(Self::INDEX_SCALE_DOWN_BITS));
                    mixer.set_input(3, p1_o2_sq, stretch_lut.stretch(p1_o2_sq));
                }
                if mixer_index >= 3 {
                    let p1_o3_sq = self.phase1_order3.refine(
                        last_bytes.hash03_16() as usize, p0_mix_sq, p0_mix_st,
                        self.luts.apm_lut(Self::INDEX_SCALE_DOWN_BITS));
                    mixer.set_input(4, p1_o3_sq, stretch_lut.stretch(p1_o3_sq));
                }
                let mixed = mixer.mix_all(self.luts.squash_lut());
                self.mixing_result_opt = Some(mixed);
                mixed.0.to_fix_u32()
            }
        }
    }

    pub fn update(&mut self, input_bit: Bit, contexts_count: usize,
                  last_bytes: &LastBytesCache) {
        match MODE {
            Mode::None => (),
            Mode::Light => {
                self.phase0_order0.update_predictions(
                    last_bytes.unfinished_byte().raw() as usize,
                    input_bit, 5, 5, true);
                self.phase1_order1.update_predictions(
                    last_bytes.hash01_16() as usize, input_bit, 5, 5, false);
                self.phase1_order2.update_predictions(
                    last_bytes.hash02_16() as usize, input_bit, 5, 5, false);
                self.phase1_order3.update_predictions(
                    last_bytes.hash03_16() as usize, input_bit, 5, 5, false);
            }
            Mode::Adaptive => {
                let mixers_row_index =
                    drain_full_option(&mut self.mixers_row_index_opt);
                let (left_factor, right_factor) = {
                    let (left_factor_index, right_factor_index) =
                        apm_factor_indexes(mixers_row_index,
                                           Self::INDEX_SCALE_DOWN_BITS);
                    (Self::FACTORS[left_factor_index],
                     Self::FACTORS[right_factor_index])
                };
                let mixer_index = quantize_contexts_count(contexts_count);
                self.phase0_order0.update_predictions(
                    last_bytes.unfinished_byte().raw() as usize,
                    input_bit, left_factor + 3, right_factor + 3, false);
                if mixer_index >= 1 {
                    self.phase1_order1.update_predictions(
                        last_bytes.hash01_16() as usize, input_bit,
                        left_factor, right_factor, false);
                }
                if mixer_index >= 2 {
                    self.phase1_order2.update_predictions(
                        last_bytes.hash02_16() as usize, input_bit,
                        left_factor, right_factor, false);
                }
                if mixer_index >= 3 {
                    self.phase1_order3.update_predictions(
                        last_bytes.hash03_16() as usize, input_bit,
                        left_factor, right_factor, false);
                }
                let mixing_result =
                    drain_full_option(&mut self.mixing_result_opt);
                self.mixers[mixers_row_index][mixer_index].update_and_reset(
                    input_bit, mixing_result.0, 1000,
                    self.luts.d_estimator_rates());
            }
        }
    }
}

fn quantize_contexts_count(contexts_count: usize) -> usize {
    match contexts_count {
        0 | 1 => 0,
        2 => 1,
        3 => 2,
        _ => 3,
    }
}

pub fn apm_factor_indexes(scaled_interval_index: usize,
                          scale_down_bits: u8) -> (usize, usize) {
    let middle_index = StretchedProbD::ZERO.to_interval_index(scale_down_bits);
    let (left_factor_index, right_factor_index) =
        if scaled_interval_index < middle_index {
            let factor_index = middle_index - scaled_interval_index - 1;
            (factor_index + 1, factor_index)
        } else {
            let factor_index = scaled_interval_index - middle_index;
            (factor_index, factor_index + 1)
        };
    (left_factor_index << scale_down_bits,
     right_factor_index << scale_down_bits)
}
