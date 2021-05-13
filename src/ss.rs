use rug::{rand::RandState, Integer};

pub fn gen_shares(
    x: &Integer,
    q: &Integer,
    r: &mut RandState
) -> (Integer, Integer) {
    let x_0 = Integer::from(q.random_below_ref(r));
    let x_1 = Integer::from(x - &x_0).pow_mod(&Integer::from(1), q).unwrap();
    (x_0, x_1)
}

pub fn reconstruct(
    share0: &Integer,
    share1: &Integer,
    q: &Integer
) -> Integer {
    Integer::from(share0 + share1).pow_mod(&Integer::from(1), q).unwrap()
}

pub fn gen_vec_shares(
    xs: &Vec<Integer>,
    q : &Integer,
    r : &mut RandState
) -> (Vec<Integer>, Vec<Integer>) {
    xs.iter()
      .map(|i| gen_shares(i, q, r))
      .unzip()
}

pub fn reconstruct_vec_shares(
    xs: &Vec<Integer>,
    ys: &Vec<Integer>,    
    q : &Integer
) -> Vec<Integer> {
    xs.iter()
      .zip(ys)
      .map(|(x,y)| reconstruct(x, y, q))
      .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;    
    use rug::{rand::RandState, Integer};

    #[test]
    fn ss_tests() {
        let mut rand = RandState::new();
        let mut rng = rand::thread_rng();
        let seed: u64 = rng.gen();        
        rand.seed(&Integer::from(seed));
        let q = Integer::from(65537);
        let shares = gen_shares(&Integer::new(), &q, &mut rand);
        assert_eq!(reconstruct(&shares.0, &shares.1, &q), Integer::new());
        let shares = gen_shares(&Integer::from(1), &q, &mut rand);
        assert_eq!(reconstruct(&shares.0, &shares.1, &q), Integer::from(1));
        let r = Integer::from(q.random_below_ref(&mut rand));
        let shares = gen_shares(&r, &q, &mut rand);
        assert_eq!(reconstruct(&shares.0, &shares.1, &q), r);
    }
    
    #[test]
    fn ss_vec_tests() {
        let mut rand = RandState::new();
        let mut rng = rand::thread_rng();
        let seed: u64 = rng.gen();        
        rand.seed(&Integer::from(seed));        
        let q = Integer::from(65537);
        let xs = vec![Integer::new(), Integer::from(1), Integer::from(2)];
        let shares = gen_vec_shares(&xs, &q, &mut rand);
        assert_eq!(reconstruct_vec_shares(&shares.0, &shares.1, &q), xs);
    }        
}
