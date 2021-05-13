use hex::encode;
use hpke::Serializable;
use pa::config::{Config, Follower, Proxy, Aggregator};
use pa::hpke::gen_keypair;
use rand::Rng;
use rug::Integer;
use std::{env,
          fs::File,
          io::Write,
};

fn main() -> std::io::Result<()> {
    let filename = env::args()
        .nth(1)
        .unwrap_or_else(|| "config.toml".to_string());
    
    let (privkey, pubkey) = gen_keypair();
    let proxy1 = Proxy {
        ip: "localhost".to_string(),
        port: "8080".to_string(),
        pubkey: encode(pubkey.to_bytes().to_vec()),
        privkey: encode(privkey.to_bytes().to_vec()),
        identity: "pki/proxy1.p12".to_string(),
        password: "solarwinds123".to_string(),
    };

    let (privkey, pubkey) = gen_keypair();
    let proxy2 = Proxy {
        ip: "localhost".to_string(),
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
        ip: "localhost".to_string(),
        port1: "8082".to_string(),
        port2: "8083".to_string(),
        pubkey: encode(pubkey.to_bytes().to_vec()),
        privkey: encode(privkey.to_bytes().to_vec()),
        identity: "pki/server1.p12".to_string(),
        password: "solarwinds123".to_string(),
    };

    let (privkey, pubkey) = gen_keypair();
    let agg2 = Aggregator {
        seed,
        ip: "localhost".to_string(),
        port1: "8084".to_string(),
        port2: "8085".to_string(),
        pubkey: encode(pubkey.to_bytes().to_vec()),
        privkey: encode(privkey.to_bytes().to_vec()),
        identity: "pki/server2.p12".to_string(),
        password: "solarwinds123".to_string(),
    };

    let (privkey, pubkey) = gen_keypair();
    let foll1 = Follower {
        seed: seed.clone(),
        ip: "localhost".to_string(),
        port1: "8086".to_string(),
        port2: "8087".to_string(),
        pubkey: encode(pubkey.to_bytes().to_vec()),
        privkey: encode(privkey.to_bytes().to_vec()),
        identity: "pki/follower1.p12".to_string(),
        password: "solarwinds123".to_string(),
    };

    let (privkey, pubkey) = gen_keypair();
    let foll2 = Follower {
        seed: seed.clone(),
        ip: "localhost".to_string(),
        port1: "8088".to_string(),
        port2: "8089".to_string(),
        pubkey: encode(pubkey.to_bytes().to_vec()),
        privkey: encode(privkey.to_bytes().to_vec()),
        identity: "pki/follower2.p12".to_string(),
        password: "solarwinds123".to_string(),
    };

    let (privkey, pubkey) = gen_keypair();
    let foll3 = Follower {
        seed: seed.clone(),
        ip: "localhost".to_string(),
        port1: "8090".to_string(),
        port2: "8091".to_string(),
        pubkey: encode(pubkey.to_bytes().to_vec()),
        privkey: encode(privkey.to_bytes().to_vec()),
        identity: "pki/follower3.p12".to_string(),
        password: "solarwinds123".to_string(),
    };

    let (privkey, pubkey) = gen_keypair();
    let foll4 = Follower {
        seed: seed.clone(),
        ip: "localhost".to_string(),
        port1: "8092".to_string(),
        port2: "8093".to_string(),
        pubkey: encode(pubkey.to_bytes().to_vec()),
        privkey: encode(privkey.to_bytes().to_vec()),
        identity: "pki/follower4.p12".to_string(),
        password: "solarwinds123".to_string(),
    };                

    let config = Config {
        //input_len: 1,
        //generator: Integer::from(18446744073709547520u64),
        //input_len: 3,
        //generator: Integer::from(16140876894906830148u64),
        //input_len: 7,
        //generator: Integer::from(12246964342811032984u64),
        //input_len: 15,
        //generator: Integer::from(5622992795699840333u64),
        //input_len: 31,
        //generator: Integer::from(18230451522939040741u64),
        //input_len: 63,
        //generator: Integer::from(2306707482978508949u64),        
        input_len: 127,
        generator: Integer::from(323234694403053661u64),
        //input_len: 255,
        //generator: Integer::from(9619880926759282999u64),
        //input_len: 511,
        //generator: Integer::from(8749440811404197325u64),
        //input_len: 1023,
        //generator: Integer::from(8367136883165034874u64),
        prime: Integer::from(18446744073709547521u64),
        root_cert: "pki/rootCA.pem".to_string(),
        queries: "bitvector-queries.toml".to_string(),
        proxy: vec![proxy1, proxy2],
        aggregator: vec![agg1, agg2],
        follower: vec![foll1, foll2, foll3, foll4],
    };

    let toml = toml::to_string(&config).unwrap();
    
    let mut file = File::create(filename)?;
    file.write_all(toml.as_bytes())?;
    
    Ok(())
}
