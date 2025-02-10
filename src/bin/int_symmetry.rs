extern crate demixer;

fn main() {
    println!("{}", std::mem::size_of::<demixer::history::tree::node::Node>());
    for x in 1..10 {
        let div4 = (x + 2) / 4;
        let shift1div2 = ((x + 1 >> 1) + 1) / 2;
        let negdiv4 = (-x - 2) / 4;
        let negshift1div1 = ((-x - 1 >> 1) - 1) / 2;
        println!("x {:2}, shift1div2 {:2}, div4 {:2}, negdiv4 {:2}, negshift1div1 {:2}",
                 x, shift1div2, div4, negdiv4, negshift1div1);
    }
}

// TODO move tests to demixer/tests/fixed_point_numbers.rs

#[test]
fn mul_i32_is_symmetric() {
    // TODO where it wasn't symmetric?
//    unimplemented!()
}

#[test]
fn mul_i64_is_symmetric() {
//    unimplemented!()
}

/*
  if LOW_PRECISION {
    assert asymmetric
  } else {
    assert symmetric
  }
*/
