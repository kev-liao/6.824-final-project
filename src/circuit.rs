use rug::{Assign, Integer};

#[derive(Debug, Clone)]
pub enum Gate {
    Const { val: Integer },
    Input { var: usize },
    Add   { in_l: Box<Gate>, in_r: Box<Gate> },
    Mul   { in_l: Box<Gate>, in_r: Box<Gate> },
}

impl Gate {
    pub fn eval(
        &self,
        q: &Integer,
        inputs: &Vec<Integer>
    ) -> Integer {
        let mut out = Integer::new();
        match self {
            Gate::Const { val: x } => {
                out.assign(x);
            },
            Gate::Input { var: x } => {
                out.assign(&inputs[*x]);
            },
            Gate::Add { in_l: x, in_r: y } => {
                out.assign(&x.eval(&q, &inputs) + &y.eval(&q, &inputs));
                out.pow_mod_mut(&Integer::from(1), q).unwrap();
            },
            Gate::Mul { in_l: x, in_r: y } => {
                out.assign(&x.eval(&q, &inputs) * &y.eval(&q, &inputs));
                out.pow_mod_mut(&Integer::from(1), q).unwrap();
            },
        };
        out
    }

    pub fn get_wire_vals(
        &self,
        q: &Integer,
        inputs: &Vec<Integer>
    ) -> (Integer, [Vec<Integer>; 2]) {
        let mut out = Integer::new();
        let mut us = Vec::new();
        let mut vs = Vec::new();
        match self {
            Gate::Const { val: x } => {
                out.assign(x);
            },
            Gate::Input { var: x } => {
                out.assign(&inputs[*x]);
            },                
            Gate::Add { in_l: x, in_r: y } => {
                let (x_out, mut x_pts) = x.get_wire_vals(&q, &inputs);
                let (y_out, mut y_pts) = y.get_wire_vals(&q, &inputs);
                out.assign(&x_out + &y_out);
                out.pow_mod_mut(&Integer::from(1), q).unwrap();
                us.append(&mut x_pts[0]);
                us.append(&mut y_pts[0]);
                vs.append(&mut x_pts[1]);
                vs.append(&mut y_pts[1]);
            },
            Gate::Mul { in_l: x, in_r: y } => {
                let (x_out, mut x_pts) = x.get_wire_vals(&q, &inputs);
                let (y_out, mut y_pts) = y.get_wire_vals(&q, &inputs);
                out.assign(&x_out * &y_out);
                out.pow_mod_mut(&Integer::from(1), q).unwrap();
                us.append(&mut x_pts[0]);
                us.append(&mut y_pts[0]);
                vs.append(&mut x_pts[1]);
                vs.append(&mut y_pts[1]);
                us.push(x_out);
                vs.push(y_out);                
            },
        };
        (out, [us, vs])
    }

    pub fn count_gates(&self) -> usize {
        let mut count = 0;
        match self {
            Gate::Add { in_l: x, in_r: y } => {
                count += x.count_gates();
                count += y.count_gates();
                count += 1;                
            },
            Gate::Mul { in_l: x, in_r: y } => {
                count += x.count_gates();
                count += y.count_gates();
                count += 1;
            },
            _ => { },
        };
        count
    }        

    pub fn count_muls(&self) -> usize {
        let mut count = 0;
        match self {
            Gate::Add { in_l: x, in_r: y } => {
                count += x.count_muls();
                count += y.count_muls();                
            },
            Gate::Mul { in_l: x, in_r: y } => {
                count += x.count_muls();
                count += y.count_muls();
                count += 1;
            },
            _ => {},
        };
        count
    }    
}

#[derive(Debug, Clone)]
pub struct Circuit {
    pub out_gates: Vec<Gate>,
    pub modulus : Integer,
}

impl Circuit {
    pub fn eval(&self, inputs: &Vec<Integer>) -> Vec<Integer> {
        self.out_gates
            .iter()
            .map(|c| c.eval(&self.modulus, &inputs))
            .collect()
    }

    pub fn get_wire_vals(
        &self,
        inputs: &Vec<Integer>
    ) -> (Vec<Integer>, [Vec<Integer>; 2]) {
        let mut outs = Vec::new();
        let mut uss = Vec::new();
        let mut vss = Vec::new();
        for c in self.out_gates.iter() {
            let (out, [us, vs]) = c.get_wire_vals(&self.modulus, &inputs);
            outs.push(out);
            uss.extend(us);
            vss.extend(vs);
        }
        (outs, [uss, vss])
    }

    pub fn count_gates(&self) -> usize {
        self.out_gates
            .iter()
            .map(|c| c.count_gates())
            .sum()
    }

    pub fn count_muls(&self) -> usize {
        self.out_gates
            .iter()
            .map(|c| c.count_muls())
            .sum()
    }
}

// C(x) = x * (x - 1)
pub fn bit_test(q: &Integer) -> Circuit {
    let neg_one = Integer::from(q - &Integer::from(1));
    let gate_0 = Gate::Add {
        in_l: Box::new(Gate::Input { var: 0 }),
        in_r: Box::new(Gate::Const { val: neg_one }),
    };
    let gate_1 = Gate::Mul {
        in_l: Box::new(Gate::Input { var: 0 }),
        in_r: Box::new(gate_0),
    };
    Circuit{
        out_gates: vec![gate_1],
        modulus : q.clone(),   
    }
}

// C(x_1, ..., x_l) = [x_1 * (x_1 - 1), ..., x_l * (x_l - 1)]
pub fn bitvector_test(q: Integer, l: usize) -> Circuit {
    let neg_one = Integer::from(&q - &Integer::from(1));
    let mut circs = Vec::new();
    for i in 0..l {
        let gate_0 = Gate::Add {
            in_l: Box::new(Gate::Input { var: i }),
            in_r: Box::new(Gate::Const { val: neg_one.clone() }),
        };
        let gate_1 = Gate::Mul {
            in_l: Box::new(Gate::Input { var: i }),
            in_r: Box::new(gate_0),
        };
        circs.push(gate_1);
    }
    Circuit{
        out_gates: circs,
        modulus : q,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rug::Integer;
    
    #[test]
    fn eval_bit_circuit() {
        let q = Integer::from(65537);        
        let circ = bit_test(&q);
        let zero_vec = vec![Integer::new()];
        let mut inputs = zero_vec.clone();
        assert_eq!(circ.eval(&inputs), zero_vec);
        inputs[0] = Integer::from(1);
        assert_eq!(circ.eval(&inputs), zero_vec);
        inputs[0] = Integer::from(2);        
        assert_ne!(circ.eval(&inputs), zero_vec);
    }

    #[test]
    fn eval_bitvector_circuit() {
        let q = Integer::from(65537);        
        let circ = bitvector_test(q, 2);
        let zero_vec = vec![Integer::new(); 2];
        let mut inputs = vec![Integer::new(), Integer::from(1)];
        assert_eq!(circ.eval(&inputs), zero_vec);
        inputs[0] = Integer::from(2);        
        assert_ne!(circ.eval(&inputs), zero_vec);
    }    
    
    #[test]
    fn collect_wires_bit_circuit() {
        let q = Integer::from(65537);         
        let circ = bit_test(&q);
        let mut inputs = vec![Integer::new()];
        assert_eq!(circ.out_gates[0].get_wire_vals(&q, &inputs),
                   (Integer::new(),
                    [vec![Integer::new()],
                     vec![Integer::from(65536)]])); 
        inputs[0] = Integer::from(1);
        assert_eq!(circ.out_gates[0].get_wire_vals(&q, &inputs),
                   (Integer::new(),
                    [vec![Integer::from(1)],
                     vec![Integer::new()]]));
    }

    #[test]
    fn collect_wires_bitvector_circuit() {
        let q = Integer::from(65537);         
        let circ = bitvector_test(q, 2);
        let inputs = vec![Integer::new(), Integer::from(1)];
        assert_eq!(circ.get_wire_vals(&inputs),
                   (vec![Integer::new(); 2],
                    [vec![Integer::new(), Integer::from(1)],
                     vec![Integer::from(65536), Integer::new()]])); 
    }    
}
