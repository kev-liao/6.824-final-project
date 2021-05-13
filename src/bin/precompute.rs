use pa::circuit::bitvector_test;
use pa::flpcp::{BitvectorVerifier, Context, Prover, Verifier};
use rand::Rng;        
use rug::Integer;
use std::{fs::File,
          io::Write,
};

fn main() -> std::io::Result<()> {    
    let p = Integer::from(18446744073709547521u64);
    //let input_len = 1;
    //let generator = Integer::from(18446744073709547520u64);
    //let input_len = 3;
    //let generator = Integer::from(16140876894906830148u64);
    //let input_len = 7;
    //let generator = Integer::from(12246964342811032984u64);
    //let input_len = 15;
    //let generator = Integer::from(5622992795699840333u64);
    //let input_len = 15;
    //let generator = Integer::from(5622992795699840333u64);
    //let input_len = 31;
    //let generator = Integer::from(18230451522939040741u64);
    //let input_len = 63;
    //let generator = Integer::from(2306707482978508949u64);
    let input_len = 127;
    let generator = Integer::from(323234694403053661u64);
    //let input_len = 255;
    //let generator = Integer::from(9619880926759282999u64);
    //let input_len = 511;
    //let generator = Integer::from(8749440811404197325u64);
    //let input_len = 1023;
    //let generator = Integer::from(8367136883165034874u64);
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
    let verifier = BitvectorVerifier {
        ctxt,
        seed: 1,
    };
    let (pi, _) = prover.gen_proofs();
    let query_state = verifier.gen_queries(&pi);

    let toml = toml::to_string(&query_state).unwrap();
    
    let mut file = File::create("bitvector-queries.toml")?;
    file.write_all(toml.as_bytes())?;
    
    Ok(())    
}
