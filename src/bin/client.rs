use hex::decode;
use hpke::kex::{Deserializable, KeyExchange};
use pa::{config::get_config,
         hpke::Kex,
         payload::gen_nested_encryptions,
};
use rand::Rng;
use rug::Integer;
use std::{env,
          error::Error,
          time::Instant,
};
use tokio::{io::AsyncWriteExt,
            net::TcpStream,
};

fn get_pubkey(pubkey: &String) -> <Kex as KeyExchange>::PublicKey {
    let pubkey_bytes = decode(&pubkey).unwrap();
    let pubkey = <Kex as KeyExchange>::PublicKey::from_bytes(&pubkey_bytes)
        .expect("Could not deserialize pubkey.");
    pubkey
}

fn gen_rand_inputs(i: usize, input_len: u64) -> (u32, Vec<Integer>) {
    let mut rng = rand::thread_rng();    
    let mut inputs = Vec::new();
    for _ in 0..input_len {
        inputs.push(Integer::from(rng.gen_range(0, 2)));
    }
    let index = match i {
        0 => rng.gen_range(0, 1000),
        1 => rng.gen_range(1000, 2000),
        _ => rng.gen_range(0, 2000),
    };
    (index, inputs)
}


//#[tokio::main(worker_threads = 2)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let i = env::args()
        .nth(1)
        .unwrap_or_else(|| 0.to_string());
    let i = i.parse::<usize>().unwrap();
    
    let config_dir = env::args()
        .nth(2)
        .unwrap_or_else(|| "config".to_string());

    let config = get_config(format!("{}/{}", config_dir,
                                    "config.toml")).unwrap();

    let proxy0_addr = format!("{}:{}",
                              config.proxy[0].ip,
                              config.proxy[0].port);

    let proxy1_addr = format!("{}:{}",
                              config.proxy[1].ip,
                              config.proxy[1].port);

    let proxy0_pubkey = get_pubkey(&config.proxy[0].pubkey);
    let proxy1_pubkey = get_pubkey(&config.proxy[1].pubkey);
    let agg0_pubkey = get_pubkey(&config.aggregator[0].pubkey);
    let agg1_pubkey = get_pubkey(&config.aggregator[1].pubkey);
    
    let start = Instant::now();

    let num_inputs = 500u32;

    println!("Sending {:?} inputs...", num_inputs);

    for _ in 0..num_inputs {
        let config = config.clone();
        let proxy0_pubkey = proxy0_pubkey.clone();
        let proxy1_pubkey = proxy1_pubkey.clone();        
        let agg0_pubkey = agg0_pubkey.clone();
        let agg1_pubkey = agg1_pubkey.clone();
        let mut outbound0 =
            TcpStream::connect(proxy0_addr.clone()).await?;
        let mut outbound1 =
            TcpStream::connect(proxy1_addr.clone()).await?;
        let fut = async move {
            let (index, inputs) = gen_rand_inputs(i, config.input_len);
            let msgs = gen_nested_encryptions(&config,
                                              index,
                                              &inputs,
                                              agg0_pubkey.clone(),
                                              agg1_pubkey.clone(),
                                              proxy0_pubkey.clone(),
                                              proxy1_pubkey.clone());
            outbound0.write_all(&msgs[0]).await?;
            outbound1.write_all(&msgs[1]).await?;
            Ok(()) as Result<(), Box<dyn Error>>
        };
    
        tokio::spawn(async move {
            if let Err(err) = fut.await {
                eprintln!("{:?}", err);
            }
        });
    }
    
    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);        
    Ok(())
}
