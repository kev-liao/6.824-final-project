use criterion::{criterion_group, criterion_main, Criterion};
use hex::{encode, decode};
use hpke::kex::{Deserializable, KeyExchange, Serializable};
use pa::{circuit,
         config::{Config, Proxy, Aggregator},
         hpke::{gen_keypair, Kex},
         payload::gen_nested_encryptions,
         ss,
         flpcp,
         flpcp::Verifier,
};
use rand::Rng;
use rug::{rand::RandState, Integer};
use std::collections::HashMap;

pub fn circuit_eval_benchmark(c: &mut Criterion) {
    let q = Integer::from(65537);
    let circ = circuit::bit_test(&q);
    let inputs = vec![Integer::new()];
    c.bench_function("Circuit evaluation - bit_test", |b| {
        b.iter(|| circ.eval(&inputs));
    });
}

pub fn ss_benchmark(c: &mut Criterion) {
    let mut rand = RandState::new();
    let mut rng = rand::thread_rng();
    let seed: u64 = rng.gen();        
    rand.seed(&Integer::from(seed));        
    macro_rules! ss_bench {
        ($gen_name:literal, $recon_name:literal, $q:expr) => {
            let q = Integer::from($q);
            c.bench_function($gen_name, |b| {
                b.iter(|| ss::gen_shares(&Integer::from(1), &q, &mut rand));
            });
            let shares = ss::gen_shares(&Integer::from(1), &q, &mut rand);
            c.bench_function($recon_name, |b| {
                b.iter(|| ss::reconstruct(&shares.0, &shares.1, &q));
            });           
        }
    }
    ss_bench!("SS gen q = 65537"     , "SS recon q = 65537"     , 65537     );
    ss_bench!("SS gen q = 2147483647", "SS recon q = 2147483647", 2147483647);
}

pub fn flpcp_benchmark(c: &mut Criterion) {
    let p = Integer::from(18446744073709547521u64);
    let generator = Integer::from(323234694403053661u64);
    let input_len = 100;
    let circuit = circuit::bitvector_test(p, input_len);
    let inputs = vec![Integer::from(1); input_len];
    let ctxt = flpcp::Context {
        generator,
        circuit,
    };
    let mut rng = rand::thread_rng();
    let prover = flpcp::Prover {
        ctxt: ctxt.clone(),
        inputs: inputs.clone(),
        seed: rng.gen(),
    };
    let verifier0 = flpcp::BitvectorVerifier {
        ctxt,
        seed: rng.gen(),
    };
    let verifier1 = verifier0.clone();
    c.bench_function("Generate FLPCP proof", |b| {
        b.iter(|| prover.gen_proofs());
    });    
    let (pi0, pi1) = prover.gen_proofs();
    let qs0 = verifier0.gen_queries(&pi0);
    let qs1 = verifier1.gen_queries(&pi1);    
    c.bench_function("Verify FLPCP proof", |b| {
        b.iter(|| verifier0.queries(&pi0, &qs0));
    });        
    let res0 = verifier0.queries(&pi0, &qs0);
    let res1 = verifier1.queries(&pi1, &qs1);
    c.bench_function("FLPCP decision", |b| {
        b.iter(|| verifier0.decision(&res0, &res1));
    });            
}

pub fn client_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Client computation time");
    group.sample_size(10000);
    let (privkey, pubkey) = gen_keypair();
    let proxy1 = Proxy {
        ip: "127.0.0.1".to_string(),
        port: "8080".to_string(),
        pubkey: encode(pubkey.to_bytes().to_vec()),
        privkey: encode(privkey.to_bytes().to_vec()),
        identity: "pki/proxy1.p12".to_string(),
        password: "solarwinds123".to_string(),
    };

    let (privkey, pubkey) = gen_keypair();
    let proxy2 = Proxy {
        ip: "127.0.0.1".to_string(),
        port: "8081".to_string(),        
        pubkey: encode(pubkey.to_bytes().to_vec()),
        privkey: encode(privkey.to_bytes().to_vec()),
        identity: "pki/proxy2.p12".to_string(),
        password: "solarwinds123".to_string(),
    };

    let mut rng = rand::thread_rng();        
    let seed: u64 = rng.gen();

    let (privkey, pubkey) = gen_keypair();
    let agg1 = Aggregator {
        seed: seed.clone(),
        ip: "127.0.0.1".to_string(),
        port1: "8082".to_string(),
        port2: "8083".to_string(),
        pubkey: encode(pubkey.to_bytes().to_vec()),
        privkey: encode(privkey.to_bytes().to_vec()),
        identity: "pki/agg1.p12".to_string(),
        password: "solarwinds123".to_string(),
    };

    let (privkey, pubkey) = gen_keypair();
    let agg2 = Aggregator {
        seed,
        ip: "127.0.0.1:8083".to_string(),
        port1: "8084".to_string(),
        port2: "8085".to_string(),
        pubkey: encode(pubkey.to_bytes().to_vec()),
        privkey: encode(privkey.to_bytes().to_vec()),
        identity: "pki/agg2.p12".to_string(),
        password: "solarwinds123".to_string(),
    };

    let mut generators = HashMap::new();
    generators.insert(1   , Integer::from(18446744073709547520u64));
    generators.insert(3   , Integer::from(16140876894906830148u64));
    generators.insert(7   , Integer::from(12246964342811032984u64));
    generators.insert(15  , Integer::from(5622992795699840333u64 ));
    generators.insert(31  , Integer::from(18230451522939040741u64));
    generators.insert(63  , Integer::from(2306707482978508949u64 ));
    generators.insert(127 , Integer::from(323234694403053661u64  ));
    generators.insert(255 , Integer::from(9619880926759282999u64 ));
    generators.insert(511 , Integer::from(8749440811404197325u64 ));
    generators.insert(1023, Integer::from(8367136883165034874u64 ));            

    let config = Config {
        input_len: 127,
        generator: generators.get(&127).unwrap().clone(),
        prime: Integer::from(18446744073709547521u64),
        root_cert: "pki/rootCA.pem".to_string(),
        queries: "bitvector-queries-127input-64bit.toml".to_string(),
        proxy: vec![proxy1.clone(), proxy2.clone()],
        aggregator: vec![agg1.clone(), agg2.clone()],
    };    

    let agg0_pubkey_bytes = decode(&config.aggregator[0].pubkey).unwrap();
    let agg0_pubkey = <Kex as KeyExchange>::PublicKey::from_bytes(&agg0_pubkey_bytes)
        .expect("Could not deserialize pubkey.");
    
    let agg1_pubkey_bytes = decode(&config.aggregator[1].pubkey).unwrap();
    let agg1_pubkey = <Kex as KeyExchange>::PublicKey::from_bytes(&agg1_pubkey_bytes)
        .expect("Could not deserialize pubkey.");
    
    let proxy0_pubkey_bytes = decode(&config.proxy[0].pubkey).unwrap();
    let proxy0_pubkey = <Kex as KeyExchange>::PublicKey::from_bytes(&proxy0_pubkey_bytes)
        .expect("Could not deserialize pubkey.");
    
    let proxy1_pubkey_bytes = decode(&config.proxy[1].pubkey).unwrap();
    let proxy1_pubkey = <Kex as KeyExchange>::PublicKey::from_bytes(&proxy1_pubkey_bytes)
        .expect("Could not deserialize pubkey.");

    for (input_len, generator) in &generators {
        let config = Config {
            input_len: *input_len,
            generator: generator.clone(),
            prime: Integer::from(18446744073709547521u64),
            root_cert: "pki/rootCA.pem".to_string(),
            queries: "bitvector-queries-127input-64bit.toml".to_string(),            
            proxy: vec![proxy1.clone(), proxy2.clone()],
            aggregator: vec![agg1.clone(), agg2.clone()],
        };

        let mut rng = rand::thread_rng();
        let mut inputs = Vec::new();
        for _ in 0..config.input_len {
            inputs.push(Integer::from(rng.gen_range(0, 2)));
        }
        let bench_id = format!("Client computation {}", input_len.to_string());

        group.bench_function(bench_id, |b| {        
            b.iter(|| gen_nested_encryptions(&config,
                                             0,
                                             &inputs,
                                             agg0_pubkey.clone(),
                                             agg1_pubkey.clone(),
                                             proxy0_pubkey.clone(),
                                             proxy1_pubkey.clone()));        
        });            
    }
}

criterion_group!(
    benches,
    circuit_eval_benchmark,
    ss_benchmark,
    flpcp_benchmark,
    client_computation,
);
criterion_main!(benches);
