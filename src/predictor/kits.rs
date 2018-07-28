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
use estimators::decelerating::DeceleratingEstimator;
use fixed_point::types::{FractOnlyU32, StretchedProbD};
use lut::LookUpTables;
use mixing::mixer::Mixer;
use util::indexer::Indexer;

const PREDICT_UPDATE_PAIRING_MSG: &str = "predict must be paired with update";

pub struct EstimatorsWithIndexer<Idx: Indexer> {
    estimators: Vec<DeceleratingEstimator>,
    indexer: Idx,
    index_opt: Option<usize>,
}

impl<Idx: Indexer> EstimatorsWithIndexer<Idx> {
    pub fn new(mut indexer: Idx) -> Self {
        EstimatorsWithIndexer {
            estimators: vec![DeceleratingEstimator::new();
                             indexer.get_array_size()],
            indexer,
            index_opt: None,
        }
    }

    pub fn predict<IdxSetup>(
        &mut self, luts: &LookUpTables, setup_indexer: IdxSetup)
        -> (FractOnlyU32, StretchedProbD)
        where IdxSetup: Fn(&mut Idx) -> &mut Idx {
        assert_eq!(self.index_opt, None);
        let index =
            setup_indexer(&mut self.indexer).get_array_index_and_reset();
        self.index_opt = Some(index);
        let prediction_sq = self.estimators[index].prediction();
        let prediction_st = luts.stretch_lut().stretch(prediction_sq);
        (prediction_sq, prediction_st)
    }

    pub fn update(&mut self, input_bit: Bit, luts: &LookUpTables) {
        let index = self.index_opt.expect(PREDICT_UPDATE_PAIRING_MSG);
        self.index_opt = None;
        self.estimators[index].update(input_bit, luts.d_estimator_rates());
    }
}

pub struct MixersWithIndexer<Mxr: Mixer, Idx: Indexer> {
    mixers: Vec<Mxr>,
    indexer: Idx,
    index_opt: Option<usize>,
    mixing_result_opt: Option<(FractOnlyU32, StretchedProbD)>,
}

impl<Mxr: Mixer, Idx: Indexer> MixersWithIndexer<Mxr, Idx> {
    pub fn new<MxrMaker: Fn() -> Mxr>(mixer_maker: MxrMaker,
                                      mut indexer: Idx) -> Self {
        let mut mixers = Vec::with_capacity(indexer.get_array_size());
        for _ in 0..mixers.capacity() { mixers.push(mixer_maker()); }
        MixersWithIndexer {
            mixers,
            indexer,
            index_opt: None,
            mixing_result_opt: None,
        }
    }

    pub fn pre_predict<IdxSetup>(&mut self, setup_indexer: IdxSetup)
        where IdxSetup: Fn(&mut Idx) -> &mut Idx {
        assert_eq!(self.mixing_result_opt, None);
        assert_eq!(self.index_opt, None);
        setup_indexer(&mut self.indexer);
    }

    pub fn predict<IdxSetup, AddInputs>(
        &mut self, setup_indexer: IdxSetup, add_inputs: AddInputs,
        luts: &LookUpTables) -> (FractOnlyU32, StretchedProbD)
        where IdxSetup: Fn(&mut Idx) -> &mut Idx,
              AddInputs: Fn(&mut Mxr) -> () {
        assert_eq!(self.mixing_result_opt, None);
        assert_eq!(self.index_opt, None);
        let index = setup_indexer(&mut self.indexer)
            .get_array_index_and_reset();
        self.index_opt = Some(index);
        let mixer = &mut self.mixers[index];
        add_inputs(mixer);
        let mixing_result = mixer.mix_all(luts.squash_lut());
        self.mixing_result_opt = Some(mixing_result);
        mixing_result
    }

    pub fn update(&mut self, input_bit: Bit, max_update_factor_index: u16,
                  luts: &LookUpTables) -> (FractOnlyU32, StretchedProbD) {
        let index = self.index_opt.expect(PREDICT_UPDATE_PAIRING_MSG);
        self.index_opt = None;
        let mixing_result = self.mixing_result_opt.unwrap();
        self.mixing_result_opt = None;
        let mixer = &mut self.mixers[index];
        mixer.update_and_reset(input_bit, mixing_result.0,
                               max_update_factor_index,
                               luts.d_estimator_rates());
        mixing_result
    }

    pub fn current_mixer(&self) -> &Mxr {
        assert_ne!(self.index_opt, None);
        &self.mixers[self.index_opt.unwrap()]
    }
}
