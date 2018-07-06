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
pub trait Indexer where Self: Sized {
    fn new(dimensions: Vec<usize>) -> Self {
        let mut result = Self::new_uninitialized();
        assert_eq!(dimensions.len(), result.dimensions_mut().len());
        for index in 0..dimensions.len() {
            result.dimensions_mut()[index].limit = dimensions[index];
        }
        result
    }
    fn new_uninitialized() -> Self;

    fn get_array_size(&mut self) -> usize {
        assert_eq!(self.common_mut().current_input_index, 0);
        let mut result: usize = 1;
        for dimension in self.dimensions_mut().iter() {
            result = result.checked_mul(dimension.limit).unwrap();
        }
        result
    }

    fn with_sub_index(&mut self, value: usize) -> &mut Self {
        let current_input_index = self.common_mut().current_input_index;
        assert!(current_input_index < self.inputs_num());
        assert!(value < self.dimensions_mut()[current_input_index].limit,
                "value: {}, limit: {}",
                value, self.dimensions_mut()[current_input_index].limit);
        self.dimensions_mut()[current_input_index].value = value;
        self.common_mut().current_input_index += 1;
        self
    }
    fn get_array_index_and_reset(&mut self) -> usize {
        let current_input_index = self.common_mut().current_input_index;
        assert_eq!(current_input_index, self.inputs_num());
        let mut result: usize = 0;
        for dimension in self.dimensions_mut().iter() {
            result *= dimension.limit;
            result += dimension.value;
        }
        self.common_mut().current_input_index = 0;
        result
    }

    fn dimensions_mut(&mut self) -> &mut [IndexDimension];
    fn common_mut(&mut self) -> &mut IndexCommon;
    fn inputs_num(&mut self) -> usize { self.dimensions_mut().len() }
}

pub struct IndexDimension {
    limit: usize,
    value: usize,
}

impl IndexDimension {
    fn new() -> Self {
        IndexDimension { limit: 0, value: 0 }
    }
}

pub struct IndexCommon {
    current_input_index: usize,
}

impl IndexCommon {
    fn new() -> Self {
        IndexCommon { current_input_index: 0 }
    }
}

pub struct Indexer1 {
    dimensions: [IndexDimension; 1],
    common: IndexCommon,
}

impl Indexer for Indexer1 {
    fn new_uninitialized() -> Self {
        let n = || IndexDimension::new();
        Self {
            dimensions: [n()],
            common: IndexCommon::new(),
        }
    }
    fn dimensions_mut(&mut self) -> &mut [IndexDimension] {
        &mut self.dimensions
    }
    fn common_mut(&mut self) -> &mut IndexCommon { &mut self.common }
}

pub struct Indexer2 {
    dimensions: [IndexDimension; 2],
    common: IndexCommon,
}

impl Indexer for Indexer2 {
    fn new_uninitialized() -> Self {
        let n = || IndexDimension::new();
        Self {
            dimensions: [n(), n()],
            common: IndexCommon::new(),
        }
    }
    fn dimensions_mut(&mut self) -> &mut [IndexDimension] {
        &mut self.dimensions
    }
    fn common_mut(&mut self) -> &mut IndexCommon { &mut self.common }
}

pub struct Indexer3 {
    dimensions: [IndexDimension; 3],
    common: IndexCommon,
}

impl Indexer for Indexer3 {
    fn new_uninitialized() -> Self {
        let n = || IndexDimension::new();
        Self {
            dimensions: [n(), n(), n()],
            common: IndexCommon::new(),
        }
    }
    fn dimensions_mut(&mut self) -> &mut [IndexDimension] {
        &mut self.dimensions
    }
    fn common_mut(&mut self) -> &mut IndexCommon { &mut self.common }
}

pub struct Indexer4 {
    dimensions: [IndexDimension; 4],
    common: IndexCommon,
}

impl Indexer for Indexer4 {
    fn new_uninitialized() -> Self {
        let n = || IndexDimension::new();
        Self {
            dimensions: [n(), n(), n(), n()],
            common: IndexCommon::new(),
        }
    }
    fn dimensions_mut(&mut self) -> &mut [IndexDimension] {
        &mut self.dimensions
    }
    fn common_mut(&mut self) -> &mut IndexCommon { &mut self.common }
}

pub struct Indexer5 {
    dimensions: [IndexDimension; 5],
    common: IndexCommon,
}

impl Indexer for Indexer5 {
    fn new_uninitialized() -> Self {
        let n = || IndexDimension::new();
        Self {
            dimensions: [n(), n(), n(), n(), n()],
            common: IndexCommon::new(),
        }
    }
    fn dimensions_mut(&mut self) -> &mut [IndexDimension] {
        &mut self.dimensions
    }
    fn common_mut(&mut self) -> &mut IndexCommon { &mut self.common }
}

pub struct Indexer6 {
    dimensions: [IndexDimension; 6],
    common: IndexCommon,
}

impl Indexer for Indexer6 {
    fn new_uninitialized() -> Self {
        let n = || IndexDimension::new();
        Self {
            dimensions: [n(), n(), n(), n(), n(), n()],
            common: IndexCommon::new(),
        }
    }
    fn dimensions_mut(&mut self) -> &mut [IndexDimension] {
        &mut self.dimensions
    }
    fn common_mut(&mut self) -> &mut IndexCommon { &mut self.common }
}
