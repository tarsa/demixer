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
pub mod estimator;
pub mod log2;

use self::estimator::*;
use self::log2::Log2Lut;

pub struct LookUpTables {
    log2_lut: Log2Lut,
    d_estimator_lut: DeceleratingEstimatorLut,
    d_estimator_cache: DeceleratingEstimatorCache,
}

impl LookUpTables {
    pub fn new() -> LookUpTables {
        let d_estimator_lut = DeceleratingEstimatorLut::make_default();
        let d_estimator_cache =
            DeceleratingEstimatorCache::new(&d_estimator_lut);
        LookUpTables {
            log2_lut: Log2Lut::new(),
            d_estimator_lut,
            d_estimator_cache,
        }
    }

    pub fn log2_lut(&self) -> &Log2Lut {
        &self.log2_lut
    }

    pub fn d_estimator_lut(&self) -> &DeceleratingEstimatorLut {
        &self.d_estimator_lut
    }

    pub fn d_estimator_cache(&self) -> &DeceleratingEstimatorCache {
        &self.d_estimator_cache
    }
}
