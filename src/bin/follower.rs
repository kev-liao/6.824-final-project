use dashmap::{DashMap, DashSet};
use futures::{
    future::{self, Ready},
    StreamExt,
};
use native_tls::{Certificate, Identity, TlsAcceptor, TlsConnector};
use pa::{circuit::bitvector_test,
         config::{get_config, get_queries},
         flpcp::{BitvectorVerifier, Context, QueryRes, QueryState, Verifier},
         payload::{Payload, PayloadSeed},
         rpc::{Agg, AggClient},
};
use std::{env,
          error::Error,
          fs::File,
          io::Read,
          sync::Arc,
          time::SystemTime,
};
use tarpc::{
    client,
    context,
    serde_transport::Transport,
    server::{self, Channel},
};
use rug::Integer;
use tokio::{io::AsyncReadExt,
            net::{TcpListener, TcpStream},
            sync::Mutex,
};
use tokio_native_tls::TlsStream;
use tokio_serde::formats::Bincode;
use tokio_stream::wrappers::TcpListenerStream;
use uuid::Uuid;

type Accumulators = Arc<DashMap<u32, Vec<u64>>>;
type UuidSet = Arc<DashSet<Uuid>>;
type QueryResDB = Arc<DashMap<Uuid, (u32, Vec<u64>, QueryRes)>>;

#[derive(Clone)]
struct AggServer {
    verifier: Arc<BitvectorVerifier>,
    qrs: QueryResDB,
    accs: Accumulators,
}

impl Agg for AggServer {
    type CheckProofFut = Ready<bool>;

    fn check_proof(self,
                   _: context::Context,
                   uuid: Uuid,
                   res: QueryRes
    ) -> Self::CheckProofFut {
        while !self.qrs.contains_key(&uuid) {
            //println!("Busywait\n");
        }
        let (index, x, res2) = &*self.qrs.get(&uuid).unwrap();
        let accept = self.verifier.decision(&res2,
                                            &res);
        if accept {
            if self.accs.contains_key(index) {
                self.accs.alter(index, |_, v|
                                accumulate(&v, &x, &res.q));
            } else {
                self.accs.insert(*index, x.to_vec());
            }
            println!("Accepted input: index {:?}, uuid {:?}\n", index, uuid);
        }
        future::ready(accept)
    }
}

fn get_acceptor(identity_file: &String) -> TlsAcceptor {
    let mut file = File::open(identity_file).unwrap();
    let mut identity = vec![];
    file.read_to_end(&mut identity).unwrap();        
    let identity = Identity::from_pkcs12(&identity, "solarwinds123").unwrap();
    let acceptor = TlsAcceptor::builder(identity).build().unwrap();
    acceptor
}

//TODO: Unduplicate
async fn tls_connect(
    addr: String,
    root_file: String,
    identity_file: String,
    common_name: String,
    password: String,
) -> Result<TlsStream<TcpStream>, Box<dyn Error>> {
    let socket = TcpStream::connect(addr).await?;
    let mut file = File::open(root_file).unwrap();
    let mut der = vec![];
    file.read_to_end(&mut der).unwrap();        
    let cert = Certificate::from_pem(&der)?;
    let mut file = File::open(identity_file).unwrap();
    let mut identity = vec![];
    file.read_to_end(&mut identity).unwrap();
    let identity = Identity::from_pkcs12(&identity, &password)?;
    let connector = TlsConnector::builder()
        .identity(identity)
        .add_root_certificate(cert)
        .build()?;    
    let connector = tokio_native_tls::TlsConnector::from(connector);
    let stream = connector.connect(&common_name, socket).await?;
    Ok(stream)
}

//fn decrypt_to_payload(
//    contents: &[u8],
//    privkey: &<Kex as KeyExchange>::PrivateKey
//) -> Payload {
//    let hpke_out: HPKEOut = bincode::deserialize(&contents).unwrap();
//    let decrypted_msg = hpke_decrypt(
//        privkey,
//        &hpke_out,
//        b"",
//    );
//    let payload: Payload = bincode::deserialize(&decrypted_msg).unwrap();
//    payload
//}
//
//fn decrypt_to_payloadseed(
//    contents: &[u8],
//    privkey: &<Kex as KeyExchange>::PrivateKey
//) -> PayloadSeed {
//    let hpke_out: HPKEOut = bincode::deserialize(&contents).unwrap();
//    let decrypted_msg = hpke_decrypt(
//        privkey,
//        &hpke_out,
//        b"",
//    );
//    let payload: PayloadSeed = bincode::deserialize(&decrypted_msg).unwrap();
//    payload
//}

fn accumulate(
    x: &Vec<u64>,
    y: &Vec<u64>,
    q: &Integer
) -> Vec<u64> {
    let old;
    if x.len() == y.len() {
        old = x.to_vec();
    } else {
        old = vec![0u64; y.len()];
    }
    old.iter()
     .zip(y)
     .map(|(x, y)| (Integer::from(*x) +
                    Integer::from(*y)).pow_mod(&Integer::from(1), &q)
          .unwrap()
          .to_u64()
          .unwrap())
     .collect()
}

async fn rpc_aggregator(
    i: usize,
    config_dir: String,
    verifier: Arc<BitvectorVerifier>,
    qrs: QueryResDB,
    accs: Accumulators,
) {
    println!("Started RPC server.");
    let config = get_config(format!("{}/{}",
                                    config_dir,
                                    "config.toml")).unwrap();
    let addr = format!("{}:{}",
                       config.follower[i].ip,
                       config.follower[i].port2);

    println!("RPC server listening on: {}", addr);    
    let identity_file = format!("{}/{}",
                                config_dir,
                                &config.follower[i + 1].identity); //CHECK

    let acceptor = get_acceptor(&identity_file);
    let acceptor = tokio_native_tls::TlsAcceptor::from(acceptor);

    let listener = TcpListener::bind(addr).await.unwrap();
    TcpListenerStream::new(listener)
        .filter_map(|r| future::ready(r.ok()))
        .map(|channel| async {
            let acceptor = acceptor.clone();
            let socket = acceptor.accept(channel).await.unwrap();
            let agg_server = AggServer{ verifier: verifier.clone(),
                                        qrs: qrs.clone(),
                                        accs: accs.clone() };
            let socket = Transport::from((socket, Bincode::default()));
            let server = server::BaseChannel::with_defaults(socket);
            tokio::spawn(async move {
                server.respond_with(agg_server.serve()).execute().await;
            });
        })
        .buffer_unordered(100)
        .for_each(|_| async {})
        .await;
}

async fn receiver(
    i: usize,
    config_dir: String,
    listener: TcpListener,
    acceptor: tokio_native_tls::TlsAcceptor,
    accs: Accumulators,
    uuids: UuidSet,
    verifier: Arc<BitvectorVerifier>,
    qs: Arc<QueryState>,
) -> Result<(), Box<dyn Error>> {
    let qrs: QueryResDB = Arc::new(DashMap::new());
    
    tokio::spawn(rpc_aggregator(i,
                                config_dir,
                                verifier.clone(),
                                qrs.clone(),
                                accs));

    loop {
        let (socket, _) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let uuids = uuids.clone();
        let qrs = qrs.clone();
        let verifier = verifier.clone();
        let qs = qs.clone();

        tokio::spawn(async move {
            // Check: Move out of loop?
            let mut stream = acceptor.accept(socket).await.expect("accept error");
            //let mut count = 0u32;
            loop {
                let uuids = uuids.clone();
                let qrs = qrs.clone();
                let verifier = verifier.clone();
                let qs = qs.clone();                                
                let mut buf = vec![0; 8192];                    
                match stream.read(&mut buf).await {
                    Ok(0) => (),
                    Ok(n) => {
                        //count += 1;                                                
                        //println!("{:?}", count);
                        //println!("Received {:?} bytes\n", n);
                        tokio::spawn(async move {
                            let payload: Payload = bincode::deserialize(&buf[0..n]).unwrap();
                            if !uuids.contains(&payload.uuid) {
                                uuids.insert(payload.uuid);
                                let res = verifier.queries(&payload.proof.to_integer(), &qs);
                                qrs.insert(payload.uuid, (payload.index, payload.proof.x, res));
                            }
                        });
                    }
                    Err(_) => (),
                };
            }
        });
    }        
}

async fn sender(
    i: usize,
    config_dir: String,
    listener: TcpListener,
    acceptor: tokio_native_tls::TlsAcceptor,
    accs: Accumulators,
    uuids: UuidSet,
    verifier: Arc<BitvectorVerifier>,
    qs: Arc<QueryState>,    
) -> Result<(), Box<dyn Error>> {
    let config = get_config(format!("{}/{}",
                                    config_dir,
                                    "config.toml")).unwrap();    
    let identity_file = format!("{}/{}",
                                config_dir,
                                &config.aggregator[1].identity);
    let root_file = format!("{}/{}",
                            config_dir,
                            &config.root_cert);
    let follower_addr = format!("{}:{}",
                                config.follower[i - 1].ip,
                                config.follower[i - 1].port2);
    println!("RPC client sending to: {}", follower_addr);        
    let stream = tls_connect(follower_addr,
                             root_file.clone(),
                             identity_file.clone(),
                             config.follower[i - 1].ip.clone(),
                             config.follower[i - 1].password.clone()).await.unwrap();
    let transport = Transport::from((stream, Bincode::default()));
    let client =
        AggClient::new(client::Config::default(),
                       transport).spawn()?;
    let client = Arc::new(Mutex::new(client));

    loop {
        let (socket, _) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let accs = accs.clone();
        let uuids = uuids.clone();
        let verifier = verifier.clone();
        let qs = qs.clone();            
        let q = config.prime.clone();
        let client = Arc::clone(&client);

        tokio::spawn(async move {
            let mut stream = acceptor.accept(socket).await.expect("accept error");
            //let mut count = 0u32;
            loop {
                let mut buf = vec![0; 8192];
                let accs = accs.clone();
                let uuids = uuids.clone();
                let verifier = verifier.clone();
                let qs = qs.clone();                    
                let q = q.clone();

                let client = Arc::clone(&client);
                match stream.read(&mut buf).await {
                    Ok(0) => (),
                    Ok(n) => {
                        //count += 1;
                        //println!("{:?}", count);
                        //println!("Received {:?} bytes\n", n);
                        tokio::spawn(async move {
                            let payload: PayloadSeed = bincode::deserialize(&buf[0..n]).unwrap();
                            let proof = &payload.proofseed.get_share();
                            let proofu64 = proof.to_u64();
                            if !uuids.contains(&payload.uuid) {
                                uuids.insert(payload.uuid);
                                let res = verifier.queries(&proof, &qs);
                                let mut client = client.lock().await;
                                let accept = client.check_proof(context::current(),
                                                                payload.uuid,
                                                                res).await.unwrap();
                                if accept {
                                    println!("Accepted input: index {:?}, uuid {:?}\n",
                                             payload.index, payload.uuid);
                                    if accs.contains_key(&payload.index) {
                                        accs.alter(&payload.index, |_, v|
                                                   accumulate(&v, &proofu64.x, &q));
                                    } else {
                                        accs.insert(payload.index,
                                                    proofu64.x);
                                    }
                                    if uuids.len() == 500 {
                                        println!("End time: {:?}\n", SystemTime::now());
                                    }
                                }
                            }                                
                        });
                    }
                    Err(_) => (),
                };
            }
        });
    }            
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

    let config = get_config(format!("{}/{}",
                                    config_dir,
                                    "config.toml")).unwrap();
    let addr = format!("{}:{}",
                       config.follower[i].ip,
                       config.follower[i].port1);
    let q = config.prime;

    println!("Listening on: {}", addr);
    
    let listener = TcpListener::bind(addr).await?;
    let identity_file = format!("{}/{}",
                                config_dir,
                                &config.aggregator[i % 2].identity);
    let acceptor = get_acceptor(&identity_file);
    let acceptor = tokio_native_tls::TlsAcceptor::from(acceptor);

    let accs: Accumulators = Arc::new(DashMap::new());
    let uuids: UuidSet = Arc::new(DashSet::new());

    let circuit = bitvector_test(q.clone(), config.input_len as usize);
    let ctxt = Context {
        generator: config.generator.clone(),
        circuit,
    };
    let verifier = BitvectorVerifier {
        ctxt,
        seed: config.follower[i].seed,
    };
    let verifier = Arc::new(verifier);
    let qs = get_queries(format!("{}/{}",
                                 config_dir,
                                 config.queries)).unwrap();
    let qs = Arc::new(qs);

    match i % 2 {
        0 => receiver(i,
                      config_dir,
                      listener,
                      acceptor,
                      accs,
                      uuids,
                      verifier,
                      qs).await?,
        1 => sender(i,
                    config_dir,
                    listener,
                    acceptor,
                    accs,
                    uuids,
                    verifier,
                    qs).await?,
        _ => println!("Error!\n"), // TODO: Error
    }
    Ok(())    
}
