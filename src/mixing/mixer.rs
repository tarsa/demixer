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
use DO_CHECKS;
use bit::Bit;
use estimators::decelerating::DeceleratingEstimator;
use fixed_point::{FixedPoint, FixI32, fix_i32, FixU32, FixI64};
use fixed_point::types::{
    FractOnlyI32, FractOnlyU32, StretchedProbD, StretchedProbQ, MixerWeight,
};
use lut::estimator::DeceleratingEstimatorRates;
use lut::squash::SquashLut;

const UPDATE_FACTOR_INDEX_LIMIT: u16 = DeceleratingEstimator::MAX_COUNT;

fn fixed_update_factor() -> FractOnlyI32 {
//    FractOnlyI32::new(500_000_000, 31)
    FractOnlyI32::HALF
}

pub trait Mixer where Self: MixerData {
    fn prediction_sq(&self, index: usize) -> FractOnlyU32 {
        self.assert_input_is_set(index);
        self.inputs()[index].prediction_sq
    }

    fn prediction_st(&self, index: usize) -> StretchedProbD {
        self.assert_input_is_set(index);
        self.inputs()[index].prediction_st
    }

    fn weight(&self, index: usize) -> MixerWeight {
        self.inputs()[index].weight
    }

    fn set_input(&mut self, index: usize, prediction_sq: FractOnlyU32,
                 prediction_st: StretchedProbD) {
        assert!(index < self.size());
        let usage_flag = 1u32 << index;
        assert_eq!(self.common().inputs_mask & usage_flag, 0);
        self.common_mut().inputs_mask |= usage_flag;
        self.inputs_mut()[index].prediction_sq = prediction_sq;
        self.inputs_mut()[index].prediction_st = prediction_st;
    }

    fn assert_input_is_set(&self, index: usize) {
        if DO_CHECKS {
            assert!(index < self.size());
            assert_ne!(self.common().inputs_mask & (1u32 << index), 0);
        }
    }

    fn mix_all(&self, squash_lut: &SquashLut)
               -> (FractOnlyU32, StretchedProbD) {
        assert_eq!(self.common().inputs_mask, (1u32 << self.size()) - 1);
        let mut result = StretchedProbQ::ZERO;
        for input in self.inputs().iter() {
            let weighted_input =
                fix_i32::mul_wide(&input.weight, &input.prediction_st);
            result = result.add(&weighted_input);
        }
        let result_st: StretchedProbD = result.clamped().to_fix_i32();
        let result_sq = squash_lut.squash(result_st);
        (result_sq, result_st)
    }

    fn update_and_reset(&mut self, input_bit: Bit, mix_result_sq: FractOnlyU32,
                        max_update_factor_index: u16,
                        d_estimator_rates_lut: &DeceleratingEstimatorRates) {
        assert_eq!(self.common().inputs_mask, (1u32 << self.size()) - 1);
        let dynamic_update_factor = FractOnlyI32::new(
            d_estimator_rates_lut[update_factor_index(self)].raw() as i32, 31);
        let update_factor: FractOnlyI32 = fix_i32::mul(
            &fixed_update_factor(), &dynamic_update_factor);
        match input_bit {
            Bit::Zero => {
                let error = FractOnlyU32::ONE_UNSAFE.sub(&mix_result_sq);
                let error = FractOnlyI32::new(error.raw() as i32, 31);
                let update_factor: FractOnlyI32 =
                    fix_i32::mul(&error, &update_factor);
                for input in self.inputs_mut().iter_mut() {
                    let weighted_error =
                        fix_i32::mul(&input.prediction_st, &update_factor);
                    input.weight = input.weight.add(&weighted_error).clamped();
                }
            }
            Bit::One => {
                let error = mix_result_sq;
                let error = FractOnlyI32::new(error.raw() as i32, 31);
                let update_factor: FractOnlyI32 =
                    fix_i32::mul(&error, &update_factor);
                for input in self.inputs_mut().iter_mut() {
                    let weighted_error =
                        fix_i32::mul(&input.prediction_st, &update_factor);
                    input.weight = input.weight.sub(&weighted_error).clamped();
                }
            }
        }
        assert!(update_factor_index(self) <= max_update_factor_index);
        assert!(max_update_factor_index <= UPDATE_FACTOR_INDEX_LIMIT);
        self.common_mut().update_factor_index =
            max_update_factor_index.min(update_factor_index(self) + 1);
        self.common_mut().inputs_mask = 0;
    }
}

fn update_factor_index<T: Mixer>(this: &T) -> u16 {
    this.common().update_factor_index
}

impl<T: MixerData> Mixer for T {}

#[derive(Clone)]
pub struct MixerInput {
    weight: MixerWeight,
    prediction_sq: FractOnlyU32,
    prediction_st: StretchedProbD,
}

impl MixerInput {
    const NEUTRAL: Self = {
        MixerInput {
            weight: MixerWeight::ZERO,
            prediction_sq: FractOnlyU32::HALF,
            prediction_st: StretchedProbD::ZERO,
        }
    };

    fn new_array_neutral(size: usize) -> Box<[Self]> {
        vec![MixerInput::NEUTRAL; size].into_boxed_slice()
    }
}

#[derive(Clone)]
pub struct MixerCommon {
    inputs_mask: u32,
    update_factor_index: u16,
}

impl MixerCommon {
    fn new(initial_update_factor_index: u16) -> Self {
        MixerCommon {
            inputs_mask: 0,
            update_factor_index: initial_update_factor_index,
        }
    }
}

pub trait MixerData where Self: Sized {
    fn size(&self) -> usize;
    fn inputs(&self) -> &[MixerInput];
    fn inputs_mut(&mut self) -> &mut [MixerInput];
    fn common(&self) -> &MixerCommon;
    fn common_mut(&mut self) -> &mut MixerCommon;
}

fn initialize_weights<Mixer: MixerData>(mixer: &mut Mixer, neutral: bool) {
    if !neutral {
        mixer.inputs_mut()[0].weight = MixerWeight::ONE;
    }
}

pub trait FixedSizeMixer where Self: MixerData {
    const SIZE: usize;
    fn new(initial_update_factor_index: u16, neutral: bool) -> Self {
        let mut result: Self = Self::new_neutral(initial_update_factor_index);
        assert_eq!(result.inputs().len(), Self::SIZE);
        initialize_weights(&mut result, neutral);
        result
    }
    fn new_neutral(initial_update_factor_index: u16) -> Self;
}

#[derive(Clone)]
pub struct Mixer1 {
    inputs: [MixerInput; 1],
    common: MixerCommon,
}

impl FixedSizeMixer for Mixer1 {
    const SIZE: usize = 1;
    fn new_neutral(initial_update_factor_index: u16) -> Self {
        assert!(initial_update_factor_index <= UPDATE_FACTOR_INDEX_LIMIT);
        Mixer1 {
            inputs: [MixerInput::NEUTRAL],
            common: MixerCommon::new(initial_update_factor_index),
        }
    }
}

impl MixerData for Mixer1 {
    fn size(&self) -> usize { Self::SIZE }
    fn inputs(&self) -> &[MixerInput] { &self.inputs }
    fn inputs_mut(&mut self) -> &mut [MixerInput] { &mut self.inputs }
    fn common(&self) -> &MixerCommon { &self.common }
    fn common_mut(&mut self) -> &mut MixerCommon { &mut self.common }
}

#[derive(Clone)]
pub struct Mixer2 {
    inputs: [MixerInput; 2],
    common: MixerCommon,
}

impl FixedSizeMixer for Mixer2 {
    const SIZE: usize = 2;
    fn new_neutral(initial_update_factor_index: u16) -> Self {
        assert!(initial_update_factor_index <= UPDATE_FACTOR_INDEX_LIMIT);
        let n = || MixerInput::NEUTRAL;
        Mixer2 {
            inputs: [n(), n()],
            common: MixerCommon::new(initial_update_factor_index),
        }
    }
}

impl MixerData for Mixer2 {
    fn size(&self) -> usize { Self::SIZE }
    fn inputs(&self) -> &[MixerInput] { &self.inputs }
    fn inputs_mut(&mut self) -> &mut [MixerInput] { &mut self.inputs }
    fn common(&self) -> &MixerCommon { &self.common }
    fn common_mut(&mut self) -> &mut MixerCommon { &mut self.common }
}

#[derive(Clone)]
pub struct Mixer3 {
    inputs: [MixerInput; 3],
    common: MixerCommon,
}

impl FixedSizeMixer for Mixer3 {
    const SIZE: usize = 3;
    fn new_neutral(initial_update_factor_index: u16) -> Self {
        assert!(initial_update_factor_index <= UPDATE_FACTOR_INDEX_LIMIT);
        let n = || MixerInput::NEUTRAL;
        Mixer3 {
            inputs: [n(), n(), n()],
            common: MixerCommon::new(initial_update_factor_index),
        }
    }
}

impl MixerData for Mixer3 {
    fn size(&self) -> usize { Self::SIZE }
    fn inputs(&self) -> &[MixerInput] { &self.inputs }
    fn inputs_mut(&mut self) -> &mut [MixerInput] { &mut self.inputs }
    fn common(&self) -> &MixerCommon { &self.common }
    fn common_mut(&mut self) -> &mut MixerCommon { &mut self.common }
}

#[derive(Clone)]
pub struct Mixer4 {
    inputs: [MixerInput; 4],
    common: MixerCommon,
}

impl FixedSizeMixer for Mixer4 {
    const SIZE: usize = 4;
    fn new_neutral(initial_update_factor_index: u16) -> Self {
        assert!(initial_update_factor_index <= UPDATE_FACTOR_INDEX_LIMIT);
        let n = || MixerInput::NEUTRAL;
        Mixer4 {
            inputs: [n(), n(), n(), n()],
            common: MixerCommon::new(initial_update_factor_index),
        }
    }
}

impl MixerData for Mixer4 {
    fn size(&self) -> usize { Self::SIZE }
    fn inputs(&self) -> &[MixerInput] { &self.inputs }
    fn inputs_mut(&mut self) -> &mut [MixerInput] { &mut self.inputs }
    fn common(&self) -> &MixerCommon { &self.common }
    fn common_mut(&mut self) -> &mut MixerCommon { &mut self.common }
}

#[derive(Clone)]
pub struct Mixer5 {
    inputs: [MixerInput; 5],
    common: MixerCommon,
}

impl FixedSizeMixer for Mixer5 {
    const SIZE: usize = 5;
    fn new_neutral(initial_update_factor_index: u16) -> Self {
        assert!(initial_update_factor_index <= UPDATE_FACTOR_INDEX_LIMIT);
        let n = || MixerInput::NEUTRAL;
        Mixer5 {
            inputs: [n(), n(), n(), n(), n()],
            common: MixerCommon::new(initial_update_factor_index),
        }
    }
}

impl MixerData for Mixer5 {
    fn size(&self) -> usize { Self::SIZE }
    fn inputs(&self) -> &[MixerInput] { &self.inputs }
    fn inputs_mut(&mut self) -> &mut [MixerInput] { &mut self.inputs }
    fn common(&self) -> &MixerCommon { &self.common }
    fn common_mut(&mut self) -> &mut MixerCommon { &mut self.common }
}

pub struct MixerN {
    inputs: Box<[MixerInput]>,
    common: MixerCommon,
}

impl MixerN {
    pub fn new(size: usize, initial_update_factor_index: u16,
               neutral: bool) -> Self {
        assert!(size <= 30);
        let mut result: Self =
            Self::new_neutral(size, initial_update_factor_index);
        assert_eq!(result.inputs().len(), size);
        initialize_weights(&mut result, neutral);
        result
    }

    pub fn new_neutral(size: usize, initial_update_factor_index: u16) -> Self {
        assert!(initial_update_factor_index <= UPDATE_FACTOR_INDEX_LIMIT);
        assert!(size <= 30);
        MixerN {
            inputs: MixerInput::new_array_neutral(size),
            common: MixerCommon::new(initial_update_factor_index),
        }
    }
}

impl MixerData for MixerN {
    fn size(&self) -> usize { self.inputs.len() }
    fn inputs(&self) -> &[MixerInput] { &self.inputs }
    fn inputs_mut(&mut self) -> &mut [MixerInput] { &mut self.inputs }
    fn common(&self) -> &MixerCommon { &self.common }
    fn common_mut(&mut self) -> &mut MixerCommon { &mut self.common }
}
