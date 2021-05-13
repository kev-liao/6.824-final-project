use crate::{circuit::bitvector_test,
            config::Config,
            hpke::{hpke_encrypt, Kex},
            flpcp::{Context, Proofu64, ProofSeed, Prover}
};
use hpke::kex::KeyExchange;
use rand::Rng;
use rug::Integer;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Payload {
    pub uuid: Uuid,
    pub index: u32,
    pub proof: Proofu64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PayloadSeed {
    pub uuid: Uuid,
    pub index: u32,
    pub proofseed: ProofSeed,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct InnerCiphertexts {
    pub left: Vec<u8>,
    pub right: Vec<u8>,
}

pub fn gen_payloads(
    index: u32,
    inputs: &Vec<Integer>,
    prime: &Integer,
    generator: &Integer
) -> (Payload, PayloadSeed) {
    let circuit = bitvector_test(prime.clone(), inputs.len() as usize);
    let mut rng = rand::thread_rng();
    let ctxt = Context {
        generator: generator.clone(),
        circuit,
    };
    let prover = Prover {
        ctxt: ctxt.clone(),
        inputs: inputs.clone(),
        seed: rng.gen(),
    };
    let pi = prover.gen_proof();
    let (proofseed, proof) = pi.share_seed();
    let uuid = Uuid::new_v4();
    let payload0 = Payload { uuid, index, proof };
    let payload1 = PayloadSeed { uuid, index, proofseed };
    (payload0, payload1)
}

pub fn gen_nested_encryptions(
    config: &Config,
    index: u32,
    inputs: &Vec<Integer>,
    agg0_pubkey: <Kex as KeyExchange>::PublicKey,
    agg1_pubkey: <Kex as KeyExchange>::PublicKey,
    proxy0_pubkey: <Kex as KeyExchange>::PublicKey,
    proxy1_pubkey: <Kex as KeyExchange>::PublicKey
) -> Vec<Vec<u8>> {
    let (payload0, payload1) = gen_payloads(index, inputs, &config.prime, &config.generator);

    fn hpke_helper(msg: &[u8], pubkey: &<Kex as KeyExchange>::PublicKey) -> Vec<u8> {
        let associated_data = b"";
        let hpke_out = hpke_encrypt(&msg, associated_data, &pubkey);
        bincode::serialize(&hpke_out).unwrap().to_vec()
    }

    let inner0 = hpke_helper(&bincode::serialize(&payload0).unwrap(),
                             &agg0_pubkey);
    let inner1 = hpke_helper(&bincode::serialize(&payload1).unwrap(),
                             &agg1_pubkey);

    let inners = InnerCiphertexts {
        left : inner0,
        right: inner1,
    };

    let outer0 = hpke_helper(&bincode::serialize(&inners).unwrap(),
                             &proxy0_pubkey);
    let outer1 = hpke_helper(&bincode::serialize(&inners).unwrap(),
                             &proxy1_pubkey);
    
    vec![outer0, outer1]
}
