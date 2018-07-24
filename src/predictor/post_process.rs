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
    mixers: [MixerN; 4],
    mixing_result_opt: Option<(FractOnlyU32, StretchedProbD)>,
}

impl<'a> PredictionFinalizer<'a> {
    pub fn new(luts: &'a LookUpTables) -> Self {
        let make = |contexts, precision|
            AdaptiveProbabilityMap::new(contexts, precision, luts.squash_lut());
        let mixers = [
            MixerN::new(2, 100, MixerInitializationMode::EqualSummingToOne),
            MixerN::new(3, 100, MixerInitializationMode::EqualSummingToOne),
            MixerN::new(4, 100, MixerInitializationMode::EqualSummingToOne),
            MixerN::new(5, 100, MixerInitializationMode::EqualSummingToOne),
        ];
        match MODE {
            Mode::None =>
                PredictionFinalizer {
                    luts,
                    phase0_order0: make(0, 0),
                    phase1_order1: make(0, 0),
                    phase1_order2: make(0, 0),
                    phase1_order3: make(0, 0),
                    mixers,
                    mixing_result_opt: None,
                },
            Mode::Light =>
                PredictionFinalizer {
                    luts,
                    phase0_order0: make(256, 0),
                    phase1_order1: make(256 * 256, 0),
                    phase1_order2: make(256 * 256, 0),
                    phase1_order3: make(256 * 256, 0),
                    mixers,
                    mixing_result_opt: None,
                },
            Mode::Adaptive =>
                PredictionFinalizer {
                    luts,
                    phase0_order0: make(256, 0),
                    phase1_order1: make(256 * 256, 0),
                    phase1_order2: make(256 * 256, 0),
                    phase1_order3: make(256 * 256, 0),
                    mixers,
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
                    last_bytes.current_byte() as usize, input_sq, input_st,
                    self.luts.apm_lut(0));
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
                let mixer_index = quantize_contexts_count(contexts_count);
                let mixer = &mut self.mixers[mixer_index];
                let p0_o0_sq = self.phase0_order0.refine(
                    last_bytes.current_byte() as usize, input_sq, input_st,
                    self.luts.apm_lut(0));
                let p0_mix_sq_raw = fix_u64::scaled_down(
                    input_sq.raw() as u64 * 3 + p0_o0_sq.raw() as u64, 2);
                let p0_mix_sq = FractOnlyU32::new(p0_mix_sq_raw as u32, 31);
                let p0_mix_st = stretch_lut.stretch(p0_mix_sq);
                mixer.set_input(0, input_sq, input_st);
                mixer.set_input(1, p0_o0_sq, stretch_lut.stretch(p0_o0_sq));
                if mixer_index >= 1 {
                    let p1_o1_sq = self.phase1_order1.refine(
                        last_bytes.hash01_16() as usize, p0_mix_sq, p0_mix_st,
                        self.luts.apm_lut(0));
                    mixer.set_input(2, p1_o1_sq, stretch_lut.stretch(p1_o1_sq));
                }
                if mixer_index >= 2 {
                    let p1_o2_sq = self.phase1_order2.refine(
                        last_bytes.hash02_16() as usize, p0_mix_sq, p0_mix_st,
                        self.luts.apm_lut(0));
                    mixer.set_input(3, p1_o2_sq, stretch_lut.stretch(p1_o2_sq));
                }
                if mixer_index >= 3 {
                    let p1_o3_sq = self.phase1_order3.refine(
                        last_bytes.hash03_16() as usize, p0_mix_sq, p0_mix_st,
                        self.luts.apm_lut(0));
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
                    last_bytes.current_byte() as usize, input_bit, 5, true);
                self.phase1_order1.update_predictions(
                    last_bytes.hash01_16() as usize, input_bit, 5, true);
                self.phase1_order2.update_predictions(
                    last_bytes.hash02_16() as usize, input_bit, 5, true);
                self.phase1_order3.update_predictions(
                    last_bytes.hash03_16() as usize, input_bit, 5, true);
            }
            Mode::Adaptive => {
                assert_ne!(self.mixing_result_opt, None);
                let mixer_index = quantize_contexts_count(contexts_count);
                self.phase0_order0.update_predictions(
                    last_bytes.current_byte() as usize, input_bit, 5, false);
                if mixer_index >= 1 {
                    self.phase1_order1.update_predictions(
                        last_bytes.hash01_16() as usize, input_bit, 5, false);
                }
                if mixer_index >= 2 {
                    self.phase1_order2.update_predictions(
                        last_bytes.hash02_16() as usize, input_bit, 5, false);
                }
                if mixer_index >= 3 {
                    self.phase1_order3.update_predictions(
                        last_bytes.hash03_16() as usize, input_bit, 5, false);
                }
                self.mixers[mixer_index].update_and_reset(
                    input_bit, self.mixing_result_opt.unwrap().0, 1000,
                    self.luts.d_estimator_rates());
                self.mixing_result_opt = None;
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
