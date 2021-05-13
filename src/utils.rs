use rug::{Integer, rand::RandState};
use rug_polynomial::ModPoly;

pub fn coeffs2poly(c: &Vec<Integer>, q: &Integer) -> ModPoly {
    let mut p = ModPoly::new(q.clone());
    for (i, a) in c.iter().enumerate() {
        p.set_coefficient_ui(i, a.to_usize().unwrap());            
    }
    p
}

pub fn rejection_sample(
    q: &Integer,
    bad: &Vec<Integer>,
    rand: &mut RandState
) -> Integer {
    let mut r = Integer::from(q.random_below_ref(rand));
    while bad.contains(&r) {
        r = Integer::from(q.random_below_ref(rand));        
    }
    r
}
