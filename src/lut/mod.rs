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
pub mod apm;
pub mod cost;
pub mod estimator;
pub mod log2;
pub mod squash;
pub mod stretch;

use self::apm::ApmWeightingLut;
use self::cost::CostTrackersLut;
use self::estimator::{
    DeceleratingEstimatorRates, DeceleratingEstimatorCache,
    DeceleratingEstimatorPredictions,
};
use self::log2::Log2Lut;
use self::squash::SquashLut;
use self::stretch::StretchLut;

pub struct LookUpTables {
    log2_lut: Log2Lut,
    d_estimator_rates: DeceleratingEstimatorRates,
    d_estimator_cache: DeceleratingEstimatorCache,
    direct_predictions: DeceleratingEstimatorPredictions,
    cost_trackers_lut: CostTrackersLut,
    stretch_lut: StretchLut,
    squash_lut: SquashLut,
    apm_luts: [ApmWeightingLut;
        LookUpTables::APM_LUTS_MAX_STRETCHED_FRACT_INDEX_BITS as usize + 1],
}

impl LookUpTables {
    pub const APM_LUTS_MAX_STRETCHED_FRACT_INDEX_BITS: u8 = 2;

    pub fn new() -> LookUpTables {
        let log2_lut = Log2Lut::new();
        let d_estimator_rates = DeceleratingEstimatorRates::make_default();
        let d_estimator_cache =
            DeceleratingEstimatorCache::new(&d_estimator_rates);
        let cost_trackers_lut =
            CostTrackersLut::new(&log2_lut, &d_estimator_rates);
        let stretch_lut = StretchLut::new(false);
        let squash_lut = SquashLut::new(&stretch_lut, false);
        let direct_predictions = DeceleratingEstimatorPredictions::new(
            &stretch_lut, &d_estimator_rates);
        let apm_luts = [
            ApmWeightingLut::new(0, &squash_lut),
            ApmWeightingLut::new(1, &squash_lut),
            ApmWeightingLut::new(2, &squash_lut),
        ];
        LookUpTables {
            log2_lut,
            d_estimator_rates,
            d_estimator_cache,
            direct_predictions,
            cost_trackers_lut,
            stretch_lut,
            squash_lut,
            apm_luts,
        }
    }

    pub fn log2_lut(&self) -> &Log2Lut {
        &self.log2_lut
    }

    pub fn d_estimator_rates(&self) -> &DeceleratingEstimatorRates {
        &self.d_estimator_rates
    }

    pub fn d_estimator_cache(&self) -> &DeceleratingEstimatorCache {
        &self.d_estimator_cache
    }

    pub fn direct_predictions(&self) -> &DeceleratingEstimatorPredictions {
        &self.direct_predictions
    }

    pub fn cost_trackers_lut(&self) -> &CostTrackersLut {
        &self.cost_trackers_lut
    }

    pub fn stretch_lut(&self) -> &StretchLut {
        &self.stretch_lut
    }

    pub fn squash_lut(&self) -> &SquashLut {
        &self.squash_lut
    }

    pub fn apm_lut(&self, stretched_fract_index_bits: u8) -> &ApmWeightingLut {
        assert!(stretched_fract_index_bits <=
            Self::APM_LUTS_MAX_STRETCHED_FRACT_INDEX_BITS);
        &self.apm_luts[stretched_fract_index_bits as usize]
    }
}
