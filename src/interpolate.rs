use rug::Integer;

pub fn eval_lagrange_basis(
    xs: &Vec<Integer>,
    p: &Integer,
    r: &Integer,
    j: usize,
) -> Integer {
    let mut prod = Integer::from(1);
    for (m, x) in xs.iter().enumerate() {
        if m != j {
            let n = r.clone() - x;            
            let mut d = xs[j].clone() - x;
            d = match d.invert(&p) {
                Ok(inverse) => inverse,
                Err(_) => unreachable!(),
            };
            prod *= n * d;
        }
    }
    prod.pow_mod_mut(&Integer::from(1), &p).unwrap();
    prod
}

#[cfg(test)]
mod tests {
    use super::*;
    use rug::Integer;
    
    #[test]
    fn eval_lagrange() {
        let q = Integer::from(37);
        let w = Integer::from(31);
        let xs: Vec<Integer> = (0..4)
            .map(|i| w.clone().pow_mod(&Integer::from(i), &q).unwrap())
            .collect();
        let r = Integer::from(11);
        assert_eq!(eval_lagrange_basis(&xs, &q, &r, 0), Integer::from(33));
        assert_eq!(eval_lagrange_basis(&xs, &q, &r, 1), Integer::from(25));
        assert_eq!(eval_lagrange_basis(&xs, &q, &r, 2), Integer::from(28));
        assert_eq!(eval_lagrange_basis(&xs, &q, &r, 3), Integer::from(26));
    }
}
