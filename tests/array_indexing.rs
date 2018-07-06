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

use demixer::util::indexer::{
    Indexer, Indexer1, Indexer2, Indexer3, Indexer4, Indexer5, Indexer6,
};

#[test]
fn indexer_can_be_reused() {
    let mut indexer = Indexer2::new(vec![3, 5]);
    assert_ne!(
        indexer.with_sub_index(2).with_sub_index(3).get_array_index_and_reset(),
        indexer.with_sub_index(1).with_sub_index(1).get_array_index_and_reset(),
    )
}

#[test]
fn last_sub_index_has_least_impact_on_final_index() {
    let make_unfinished = || {
        let mut indexer = Indexer5::new(vec![5, 8, 9, 1, 3]);
        indexer
            .with_sub_index(2)
            .with_sub_index(3)
            .with_sub_index(4)
            .with_sub_index(0);
        indexer
    };
    assert_eq!(
        make_unfinished().with_sub_index(1).get_array_index_and_reset() + 1,
        make_unfinished().with_sub_index(2).get_array_index_and_reset()
    );
}

#[test]
fn maximum_sub_indices_lead_to_maximum_final_index() {
    let make2 = || Indexer2::new(vec![5, 8]);
    assert_eq!(make2()
                   .with_sub_index(4)
                   .with_sub_index(7)
                   .get_array_index_and_reset() + 1,
               make2().get_array_size());
    let make5 = || Indexer5::new(vec![6, 2, 3, 1, 6]);
    assert_eq!(make5()
                   .with_sub_index(5)
                   .with_sub_index(1)
                   .with_sub_index(2)
                   .with_sub_index(0)
                   .with_sub_index(5)
                   .get_array_index_and_reset() + 1,
               make5().get_array_size());
}

#[test]
fn indexer_computes_correct_array_size() {
    assert_eq!(Indexer1::new(vec![5]).get_array_size(), 5);
    assert_eq!(Indexer2::new(vec![4, 5]).get_array_size(), 20);
    assert_eq!(Indexer3::new(vec![2, 6, 3]).get_array_size(), 36);
    assert_eq!(Indexer4::new(vec![2, 4, 6, 3]).get_array_size(), 144);
    assert_eq!(Indexer5::new(vec![5, 8, 9, 1, 3]).get_array_size(), 1080);
    assert_eq!(Indexer6::new(vec![5, 3, 8, 1, 3, 9]).get_array_size(), 3240);
}

#[test]
fn indexer_computes_correct_array_index() {
    let index = Indexer1::new(vec![5])
        .with_sub_index(3)
        .get_array_index_and_reset();
    assert_eq!(index, 3);

    let index = Indexer2::new(vec![4, 5])
        .with_sub_index(2)
        .with_sub_index(3)
        .get_array_index_and_reset();
    assert_eq!(index, 13);

    let index = Indexer3::new(vec![2, 6, 3])
        .with_sub_index(0)
        .with_sub_index(4)
        .with_sub_index(1)
        .get_array_index_and_reset();
    assert_eq!(index, 13);

    let index = Indexer4::new(vec![2, 4, 6, 3])
        .with_sub_index(1)
        .with_sub_index(3)
        .with_sub_index(3)
        .with_sub_index(1)
        .get_array_index_and_reset();
    assert_eq!(index, 136);

    let index = Indexer5::new(vec![5, 8, 9, 1, 3])
        .with_sub_index(2)
        .with_sub_index(3)
        .with_sub_index(4)
        .with_sub_index(0)
        .with_sub_index(2)
        .get_array_index_and_reset();
    assert_eq!(index, 527);

    let index = Indexer6::new(vec![5, 3, 8, 1, 3, 9])
        .with_sub_index(1)
        .with_sub_index(2)
        .with_sub_index(3)
        .with_sub_index(0)
        .with_sub_index(1)
        .with_sub_index(8)
        .get_array_index_and_reset();
    assert_eq!(index, 1178);
}
