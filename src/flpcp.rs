use crate::circuit::Circuit;
use crate::interpolate::eval_lagrange_basis;
use crate::ss::{gen_vec_shares, reconstruct, reconstruct_vec_shares};    
use crate::utils::{coeffs2poly, rejection_sample};
use rand::Rng;    
use rug::{Integer, rand::RandState};
use rug_polynomial::ModPoly;
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq)]
pub struct Proof {
    pub x: Vec<Integer>,
    pub w: Vec<Integer>,    
    pub z: Vec<Integer>,
    pub c: Vec<Integer>,
    pub q: Integer,
}

impl Proof {
    pub fn len(&self) -> usize {
        self.x.len() + self.w.len() + self.z.len() + self.c.len()
    }

    pub fn collect(&self) -> Vec<Integer> {
        let mut pi = self.x.clone();
        pi.extend(self.w.clone());
        pi.extend(self.z.clone());
        pi.extend(self.c.clone());
        pi
    }

    pub fn share(&self, r: &mut RandState) -> (Proof, Proof) {
        let (x0, x1) = gen_vec_shares(&self.x, &self.q, r);
        let (w0, w1) = gen_vec_shares(&self.w, &self.q, r);
        let (z0, z1) = gen_vec_shares(&self.z, &self.q, r);
        let (c0, c1) = gen_vec_shares(&self.c, &self.q, r);
        let pi0 = Proof { x: x0, w: w0, z: z0, c: c0, q: self.q.clone() };
        let pi1 = Proof { x: x1, w: w1, z: z1, c: c1, q: self.q.clone() };
        (pi0, pi1)
    }

    pub fn reconstruct(&self, other: &Proof) -> Proof {
        let x = reconstruct_vec_shares(&self.x, &other.x, &self.q);
        let w = reconstruct_vec_shares(&self.w, &other.w, &self.q);
        let z = reconstruct_vec_shares(&self.z, &other.z, &self.q);
        let c = reconstruct_vec_shares(&self.c, &other.c, &self.q);
        let pi = Proof { x, w, z, c, q: self.q.clone() };
        pi
    }    

    pub fn share_seed(&self) -> (ProofSeed, Proofu64) {
        let mut rand = RandState::new();
        let mut rng = rand::thread_rng();
        let seed: u64 = rng.gen();
        rand.seed(&Integer::from(seed));
        let (_, x) = gen_vec_shares(&self.x, &self.q, &mut rand);
        let (_, w) = gen_vec_shares(&self.w, &self.q, &mut rand);
        let (_, z) = gen_vec_shares(&self.z, &self.q, &mut rand);
        let (_, c) = gen_vec_shares(&self.c, &self.q, &mut rand);
        let ps = ProofSeed { seed,
                             x_len: x.len() as u16,
                             w_len: w.len() as u16,
                             z_len: z.len() as u16,
                             c_len: c.len() as u16,
                             q    : self.q.to_u64().unwrap(),
        };
        let pi = Proof { x, w, z, c, q: self.q.clone() };
        (ps, pi.to_u64())
    }

    pub fn query(&self, query: &Query, q: &Integer) -> Integer {
        assert!(self.len() == query.vec.len());
        let mut res: Integer = self
            .collect()
            .iter()
            .zip(&query.vec)
            .map(|(x, y)| x * y)
            .sum();
        res += &query.scalar;
        res.pow_mod_mut(&Integer::from(1), &q).unwrap();
        res
    }

    pub fn to_u64(&self) -> Proofu64 {
        Proofu64 {
            x: self.x.iter().map(|x| x.to_u64().unwrap()).collect(),
            w: self.w.iter().map(|x| x.to_u64().unwrap()).collect(),
            z: self.z.iter().map(|x| x.to_u64().unwrap()).collect(),
            c: self.c.iter().map(|x| x.to_u64().unwrap()).collect(),
            q: self.q.to_u64().unwrap(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Proofu64 {
    pub x: Vec<u64>,
    pub w: Vec<u64>,    
    pub z: Vec<u64>,
    pub c: Vec<u64>,
    pub q: u64,
}

impl Proofu64 {
    pub fn to_integer(&self) -> Proof {
        Proof {
            x: self.x.iter().map(|x| Integer::from(*x)).collect(),
            w: self.w.iter().map(|x| Integer::from(*x)).collect(),
            z: self.z.iter().map(|x| Integer::from(*x)).collect(),
            c: self.c.iter().map(|x| Integer::from(*x)).collect(),
            q: Integer::from(self.q),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ProofSeed {
    pub seed: u64,
    pub x_len: u16,
    pub w_len: u16,    
    pub z_len: u16,
    pub c_len: u16,
    pub q: u64,    
}

impl ProofSeed {
    pub fn get_share(&self) -> Proof {
        let mut rand = RandState::new();
        rand.seed(&Integer::from(self.seed));
        let q = Integer::from(self.q);
        let x = (0..self.x_len)
            .map(|_| Integer::from(q.random_below_ref(&mut rand)))
            .collect();
        let w = (0..self.w_len)
            .map(|_| Integer::from(q.random_below_ref(&mut rand)))
            .collect();
        let z = (0..self.z_len)
            .map(|_| Integer::from(q.random_below_ref(&mut rand)))
            .collect();
        let c = (0..self.c_len)
            .map(|_| Integer::from(q.random_below_ref(&mut rand)))
            .collect();
        let pi = Proof { x, w, z, c, q };
        pi
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub vec: Vec<Integer>,
    pub scalar: Integer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRes {
    pub a : Integer,
    pub a1: Integer,
    pub d1: Integer,
    pub a2: Integer,
    pub d2: Integer,    
    pub b : Integer,
    pub q : Integer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryState {
    pub r : Integer,
    pub xs: Vec<Integer>,
    pub q0: Query,
    pub q1: Query,
    pub q2: Query,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub generator: Integer,
    pub circuit: Circuit,
}

#[derive(Debug, Clone)]
pub struct Prover {
    pub ctxt: Context,
    pub inputs: Vec<Integer>,
    pub seed: u64,
}

impl Prover {
    // TODO: Unduplicate
    pub fn gen_proof(&self) -> Proof {
        let Context {generator: w, circuit: c} = &self.ctxt;
        let q = &c.modulus;
        let mut rand = RandState::new();
        rand.seed(&Integer::from(self.seed));        
        let (_, [mut us, mut vs]) = c.get_wire_vals(&self.inputs);
        let z = vec![Integer::from(q.random_below_ref(&mut rand)),
                     Integer::from(q.random_below_ref(&mut rand))];
        us.insert(0, z[0].clone());
        vs.insert(0, z[1].clone());        
        let extension = us.len().next_power_of_two() - us.len();
        us.extend(vec![Integer::new(); extension]);
        vs.extend(vec![Integer::new(); extension]);
        let f0 = ModPoly::interpolate_from_mul_subgroup(us.clone(), q.clone(), w);
        let f1 = ModPoly::interpolate_from_mul_subgroup(vs.clone(), q.clone(), w);
        let p = &f0 * f1;
        let mut c_p = Vec::new();
        for i in 0..p.len() {
            c_p.push(p.get_coefficient(i).pow_mod(&Integer::from(1), q).unwrap());
        }
        let max_coeffs = (us.len() - 1) * 2 + 1;
        c_p.extend(vec![Integer::new(); max_coeffs - c_p.len()]);
        let pi = Proof {
            x: self.inputs.to_vec(),
            w: vec![],
            z: z,
            c: c_p,
            q: q.clone(),
        };
        pi
    }
    
    pub fn gen_proofs(&self) -> (Proof, Proof) {
        let Context {generator: w, circuit: c} = &self.ctxt;
        let q = &c.modulus;
        let mut rand = RandState::new();
        rand.seed(&Integer::from(self.seed));        
        let (_, [mut us, mut vs]) = c.get_wire_vals(&self.inputs);
        let z = vec![Integer::from(q.random_below_ref(&mut rand)),
                     Integer::from(q.random_below_ref(&mut rand))];
        us.insert(0, z[0].clone());
        vs.insert(0, z[1].clone());        
        let extension = us.len().next_power_of_two() - us.len();
        us.extend(vec![Integer::new(); extension]);
        vs.extend(vec![Integer::new(); extension]);
        let f0 = ModPoly::interpolate_from_mul_subgroup(us.clone(), q.clone(), w);
        let f1 = ModPoly::interpolate_from_mul_subgroup(vs.clone(), q.clone(), w);
        let p = &f0 * f1;
        let mut c_p = Vec::new();
        for i in 0..p.len() {
            c_p.push(p.get_coefficient(i).pow_mod(&Integer::from(1), q).unwrap());
        }
        let max_coeffs = (us.len() - 1) * 2 + 1;
        c_p.extend(vec![Integer::new(); max_coeffs - c_p.len()]);
        let pi = Proof {
            x: self.inputs.to_vec(),
            w: vec![],
            z: z,
            c: c_p,
            q: q.clone(),
        };
        pi.share(&mut rand)
    }
}

pub trait Verifier {
    fn gen_queries(&self, pi: &Proof) -> QueryState;

    fn queries(&self, pi: &Proof, qs: &QueryState) -> QueryRes {
        let q = pi.q.clone();        
        let p = coeffs2poly(&pi.c, &q);
        let a = p.evaluate(&qs.r);        
        let a1 = pi.query(&qs.q1, &q);
        let d1 = qs.q1.scalar.clone();        
        let a2 = pi.query(&qs.q2, &q);
        let d2 = qs.q2.scalar.clone();
        let b = pi.query(&qs.q0, &q);
        QueryRes { a, a1, d1, a2, d2, b, q }
    }

    fn decision(&self, res0: &QueryRes, res1: &QueryRes) -> bool {
        assert!(res0.q == res1.q);
        let q  = &res0.q;
        let a  = reconstruct(&res0.a , &res1.a , q);
        let a1 = Integer::from(&res0.a1 - &res0.d1) +  &res1.a1;
        let a2 = Integer::from(&res0.a2 - &res0.d2) + &res1.a2;
        let b  = reconstruct(&res0.b , &res1.b , q);
        let a1a2 = (a1 * a2).pow_mod(&Integer::from(1), q).unwrap();
        a1a2 == a && b == Integer::new()
    }        
}

#[derive(Debug, Clone)]
pub struct BitvectorVerifier {
    pub ctxt: Context,
    pub seed: u64,    
}

impl Verifier for BitvectorVerifier {
    fn gen_queries(&self, pi: &Proof) -> QueryState {
        let Context {generator: w, circuit: c} = &self.ctxt;
        let q = &c.modulus;
        let mut rand = RandState::new();
        rand.seed(&Integer::from(self.seed));
        
        // FLPCP queries
        let num_pts = (c.count_muls() + 1).next_power_of_two();
        let xs: Vec<Integer> = (0..num_pts)
            .map(|i| w.clone().pow_mod(&Integer::from(i), &q).unwrap())
            .collect();
        let r = rejection_sample(&q, &xs, &mut rand);

        let mut query = Vec::new();
        for i in 1..pi.x.len() + 1 {
            query.push(eval_lagrange_basis(&xs, &q, &r, i));
        }
        query.push(eval_lagrange_basis(&xs, &q, &r, 0));
        query.extend(vec![Integer::new(); pi.c.len() + 1]);
        let d1 = Integer::new();        
        let q1 = Query {
            vec: query,
            scalar: d1.clone(),
        };

        let mut query = Vec::new();
        for i in 1..pi.x.len() + 1 {
            query.push(eval_lagrange_basis(&xs, &q, &r, i));
        }
        query.push(Integer::new());
        query.push(eval_lagrange_basis(&xs, &q, &r, 0));
        query.extend(vec![Integer::new(); pi.c.len()]);
        let mut d2 = Integer::new();
        for i in 0..pi.x.len() {
            d2 += &query[i];
        }
        d2 = -d2;
        d2.pow_mod_mut(&Integer::from(1), &q).unwrap();        
        let q2 = Query {
            vec: query,
            scalar: d2.clone(),
        };

        // Compute b = p(M)
        let mut rs = Vec::new();
        for _ in 0..pi.x.len() {
            rs.push(Integer::from(q.random_below_ref(&mut rand)));
        }
        let mut query = vec![Integer::new(); pi.x.len() + 2];
        for i in 0..pi.c.len() {
            let mut sum = w.clone().pow_mod(&Integer::from(i), &q).unwrap() * &rs[0];
            for j in 1..rs.len() {
                sum += w.clone().pow_mod(&Integer::from(i * (j + 1)), &q).unwrap() * &rs[j];
            }
            query.push(sum);
        }
        let q0 = Query {
            vec: query,
            scalar: Integer::new(),
        };

        QueryState { r, xs, q0, q1, q2 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::*;
    use rand::Rng;        
    use rug::Integer;

    #[test]    
    fn flpcp_32_bit_accept() {
        let p = Integer::from(4293918721u64);
        let generator = Integer::from(2960092488u64);
        let input_len = 127;
        let circuit = bitvector_test(p, input_len);
        let inputs = vec![Integer::from(1); input_len];
        let ctxt = Context {
            generator,
            circuit,
        };
        let mut rng = rand::thread_rng();
        let prover = Prover {
            ctxt: ctxt.clone(),
            inputs,
            seed: rng.gen(),
        };
        let verifier0 = BitvectorVerifier {
            ctxt,
            seed: rng.gen(),
        };
        let verifier1 = verifier0.clone();
        let (pi0, pi1) = prover.gen_proofs();
        let qs0 = verifier0.gen_queries(&pi0);
        let qs1 = verifier1.gen_queries(&pi1);
        assert_eq!(qs0, qs1);
        let res0 = verifier0.queries(&pi0, &qs0);
        let res1 = verifier1.queries(&pi1, &qs1);
        assert!(verifier0.decision(&res0, &res1));
    }

    #[test]    
    fn flpcp_32_bit_reject() {
        let p = Integer::from(4293918721u64);
        let generator = Integer::from(2960092488u64);
        let input_len = 127;
        let circuit = bitvector_test(p, input_len);
        let inputs = vec![Integer::from(1); input_len];
        let ctxt = Context {
            generator,
            circuit,
        };
        let mut rng = rand::thread_rng();
        let prover = Prover {
            ctxt: ctxt.clone(),
            inputs,
            seed: rng.gen(),
        };
        let verifier0 = BitvectorVerifier {
            ctxt,
            seed: rng.gen(),
        };
        let verifier1 = verifier0.clone();
        let (pi0, pi1) = prover.gen_proofs();
        let qs0 = verifier0.gen_queries(&pi0);
        let qs1 = verifier1.gen_queries(&pi1);
        assert_eq!(qs0, qs1);
        let res0 = verifier0.queries(&pi0, &qs0);
        let res1 = verifier1.queries(&pi1, &qs1);
        assert!(!verifier0.decision(&res0, &res1));
    }

    #[test]    
    fn flpcp_64_bit_accept() {
        let p = Integer::from(18446744073709547521u64);
        let generator = Integer::from(323234694403053661u64);
        let input_len = 127;
        let circuit = bitvector_test(p, input_len);
        let inputs = vec![Integer::from(1); input_len];
        let ctxt = Context {
            generator,
            circuit,
        };
        let mut rng = rand::thread_rng();
        let prover = Prover {
            ctxt: ctxt.clone(),
            inputs,
            seed: rng.gen(),
        };
        let verifier0 = BitvectorVerifier {
            ctxt,
            seed: rng.gen(),
        };
        let verifier1 = verifier0.clone();
        let (pi0, pi1) = prover.gen_proofs();
        let qs0 = verifier0.gen_queries(&pi0);
        let qs1 = verifier1.gen_queries(&pi1);
        assert_eq!(qs0, qs1);
        let res0 = verifier0.queries(&pi0, &qs0);
        let res1 = verifier1.queries(&pi1, &qs1);
        assert!(verifier0.decision(&res0, &res1));
    }

    #[test]    
    fn flpcp_64_bit_reject() {
        let p = Integer::from(18446744073709547521u64);
        let generator = Integer::from(323234694403053661u64);
        let input_len = 127;
        let circuit = bitvector_test(p, input_len);
        let inputs = vec![Integer::from(2); input_len];
        let ctxt = Context {
            generator,
            circuit,
        };
        let mut rng = rand::thread_rng();
        let prover = Prover {
            ctxt: ctxt.clone(),
            inputs,
            seed: rng.gen(),
        };
        let verifier0 = BitvectorVerifier {
            ctxt,
            seed: rng.gen(),
        };
        let verifier1 = verifier0.clone();
        let (pi0, pi1) = prover.gen_proofs();
        let qs0 = verifier0.gen_queries(&pi0);
        let qs1 = verifier1.gen_queries(&pi1);
        assert_eq!(qs0, qs1);
        let res0 = verifier0.queries(&pi0, &qs0);
        let res1 = verifier1.queries(&pi1, &qs1);
        assert!(!verifier0.decision(&res0, &res1));
    }

    #[test]    
    fn share_seed_proof() {
        let p = Integer::from(4293918721u64);
        let generator = Integer::from(2960092488u64);
        let input_len = 127;
        let circuit = bitvector_test(p, input_len);
        let inputs = vec![Integer::from(1); input_len];
        let ctxt = Context {
            generator,
            circuit,
        };
        let mut rng = rand::thread_rng();
        let prover = Prover {
            ctxt: ctxt.clone(),
            inputs,
            seed: rng.gen(),
        };
        let pi0 = prover.gen_proof();
        let (ps, piu64) = pi0.share_seed();
        let pi1 = ps.get_share().reconstruct(&piu64.to_integer());
        assert_eq!(pi0, pi1);
    }    
}
