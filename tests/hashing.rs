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

use demixer::util::hash::Fnv1A;

#[test]
fn initial_state() {
    assert_eq!(Fnv1A::new().into_u64(), 0xcbf29ce484222325);
}

#[test]
fn test_vectors_64_bit() {
    assert_eq!(hash_bytes_64(&[0x00]), 0xaf63bd4c8601b7df);
    assert_eq!(hash_bytes_64(&[0x01]), 0xaf63bc4c8601b62c);
    assert_eq!(hash_bytes_64(&[0xb5]), 0xaf64284c86026db0);
    assert_eq!(hash_bytes_64(&[0xff]), 0xaf64724c8602eb6e);

    assert_eq!(hash_bytes_64(&[0x00, 0x00]), 0x08328807b4eb6fed);
    assert_eq!(hash_bytes_64(&[0x00, 0x01]), 0x08328707b4eb6e3a);
    assert_eq!(hash_bytes_64(&[0x01, 0x00]), 0x082f2207b4e88cc4);
    assert_eq!(hash_bytes_64(&[0x01, 0x01]), 0x082f2307b4e88e77);

    assert_eq!(hash_bytes_64(&[0x00, 0xb5, 0xff]), 0xd7c037186abea953);
    assert_eq!(hash_bytes_64(&[0xb5, 0xff, 0x00]), 0x76d9d51a77ee8ea7);
    assert_eq!(hash_bytes_64(&[0xff, 0x00, 0xb5]), 0xf9207e1be415526d);
}

#[test]
fn test_vectors_32_bit() {
    assert_eq!(hash_bytes_32(&[0x00]), 0x29620a93);
    assert_eq!(hash_bytes_32(&[0x01]), 0x29620a60);
    assert_eq!(hash_bytes_32(&[0xb5]), 0x296645fc);
    assert_eq!(hash_bytes_32(&[0xff]), 0x29669922);

    assert_eq!(hash_bytes_32(&[0x00, 0x00]), 0xbcd9e7ea);
    assert_eq!(hash_bytes_32(&[0x00, 0x01]), 0xbcd9e93d);
    assert_eq!(hash_bytes_32(&[0x01, 0x00]), 0xbcc7aec3);
    assert_eq!(hash_bytes_32(&[0x01, 0x01]), 0xbcc7ad70);

    assert_eq!(hash_bytes_32(&[0x00, 0xb5, 0xff]), 0xbd7e9e4b);
    assert_eq!(hash_bytes_32(&[0xb5, 0xff, 0x00]), 0x01375bbd);
    assert_eq!(hash_bytes_32(&[0xff, 0x00, 0xb5]), 0x1d352c76);
}

#[test]
fn test_vectors_16_bit() {
    assert_eq!(hash_bytes_16(&[0x00]), 0x23f1);
    assert_eq!(hash_bytes_16(&[0x01]), 0x2302);
    assert_eq!(hash_bytes_16(&[0xb5]), 0x6c9a);
    assert_eq!(hash_bytes_16(&[0xff]), 0xb044);

    assert_eq!(hash_bytes_16(&[0x00, 0x00]), 0x5b33);
    assert_eq!(hash_bytes_16(&[0x00, 0x01]), 0x55e4);
    assert_eq!(hash_bytes_16(&[0x01, 0x00]), 0x1204);
    assert_eq!(hash_bytes_16(&[0x01, 0x01]), 0x11b7);

    assert_eq!(hash_bytes_16(&[0x00, 0xb5, 0xff]), 0x2335);
    assert_eq!(hash_bytes_16(&[0xb5, 0xff, 0x00]), 0x5a8a);
    assert_eq!(hash_bytes_16(&[0xff, 0x00, 0xb5]), 0x3143);
}

fn hash_bytes_64(array: &[u8]) -> u64 {
    hash_bytes(array).into_u64()
}

fn hash_bytes_32(array: &[u8]) -> u32 {
    hash_bytes(array).into_u32()
}

fn hash_bytes_16(array: &[u8]) -> u16 {
    hash_bytes(array).into_u16()
}

fn hash_bytes(array: &[u8]) -> Fnv1A {
    let mut hasher = Fnv1A::new();
    array.iter().for_each(|&x| hasher.mix_byte(x));
    hasher
}
