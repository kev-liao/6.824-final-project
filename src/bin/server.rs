use hpke::kex::KeyExchange;
use native_tls::{Certificate, Identity, TlsAcceptor, TlsConnector};
use pa::{config::get_config,
         hpke::{get_privkey, HPKEOut, hpke_decrypt, Kex},
         payload::{Payload, PayloadSeed},
};
use std::{env,
          error::Error,
          fs::File,
          io::Read,
          sync::Arc,
};
use tokio::{io::{AsyncReadExt, AsyncWriteExt},
            net::{TcpListener, TcpStream},
            sync::Mutex,
};
use tokio_native_tls::TlsStream;

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

fn decrypt_to_payload(
    contents: &[u8],
    privkey: &<Kex as KeyExchange>::PrivateKey
) -> Payload {
    let hpke_out: HPKEOut = bincode::deserialize(&contents).unwrap();
    let decrypted_msg = hpke_decrypt(
        privkey,
        &hpke_out,
        b"",
    );
    let payload: Payload = bincode::deserialize(&decrypted_msg).unwrap();
    payload
}

fn decrypt_to_payloadseed(
    contents: &[u8],
    privkey: &<Kex as KeyExchange>::PrivateKey
) -> PayloadSeed {
    let hpke_out: HPKEOut = bincode::deserialize(&contents).unwrap();
    let decrypted_msg = hpke_decrypt(
        privkey,
        &hpke_out,
        b"",
    );
    let payload: PayloadSeed = bincode::deserialize(&decrypted_msg).unwrap();
    payload
}

async fn aggregator0(
    i: usize,
    config_dir: String,
    listener: TcpListener,
    acceptor: tokio_native_tls::TlsAcceptor
) -> Result<(), Box<dyn Error>> {
    let config = get_config(format!("{}/{}",
                                    config_dir,
                                    "config.toml"))?;

    let identity_file = format!("{}/{}",
                                config_dir,
                                &config.aggregator[i].identity);
    let root_file = format!("{}/{}",
                            config_dir,
                            &config.root_cert);
    let privkey = get_privkey(&config.aggregator[i].privkey);    

    let foll0_addr = format!("{}:{}",
                            config.follower[i].ip,
                            config.follower[i].port1);    
    let foll1_addr = format!("{}:{}",
                            config.follower[i + 2].ip,
                            config.follower[i + 2].port1);        

    println!("Forwarding to: {} and {}", foll0_addr, foll1_addr);

    let stream0 = tls_connect(foll0_addr.clone(),
                              root_file.clone(),
                              identity_file.clone(),
                              config.follower[i].ip.clone(),
                              config.aggregator[i].password.clone()).await?;
    let stream0 = Arc::new(Mutex::new(stream0));
    let stream1 = tls_connect(foll1_addr.clone(),
                              root_file.clone(),
                              identity_file.clone(),
                              config.follower[i + 2].ip.clone(),
                              config.aggregator[i].password.clone()).await?;
    let stream1 = Arc::new(Mutex::new(stream1));    

    loop {
        let (socket, _) = listener.accept().await?;
        let stream0 = Arc::clone(&stream0);
        let stream1 = Arc::clone(&stream1);
        let acceptor = acceptor.clone();
        let privkey = privkey.clone();

        tokio::spawn(async move {
            // Check: Move out of loop?
            let mut stream = acceptor.accept(socket).await.expect("accept error");
            //let mut count = 0u32;
            loop {
                let stream0 = Arc::clone(&stream0);
                let stream1 = Arc::clone(&stream1);                
                let privkey = privkey.clone();
                let mut buf = vec![0; 8192];
                match stream.read(&mut buf).await {
                    Ok(0) => (),
                    Ok(n) => {
                        //count += 1;                                                
                        //println!("{:?}", count);
                        //println!("Received {:?} bytes\n", n);
                        tokio::spawn(async move {
                            let payload = decrypt_to_payload(&buf[0..n],
                                                             &privkey);
                            let payload_bytes = bincode::serialize(&payload).unwrap();
                            if payload.index >= 1000 {
                                let mut stream0 = stream0.lock().await;
                                stream0
                                    .write_all(&payload_bytes)
                                    .await
                                    .expect("Failed to write data to socket");
                            } else {
                                let mut stream1 = stream1.lock().await;
                                stream1
                                    .write_all(&payload_bytes)
                                    .await
                                    .expect("Failed to write data to socket");
                            }
                        });
                    }
                    Err(_) => (),
                };
            }
        });
    }        
}

async fn aggregator1(
    i: usize,
    config_dir: String,
    listener: TcpListener,
    acceptor: tokio_native_tls::TlsAcceptor
) -> Result<(), Box<dyn Error>> {
    let config = get_config(format!("{}/{}",
                                    config_dir,
                                    "config.toml"))?;

    let identity_file = format!("{}/{}",
                                config_dir,
                                &config.aggregator[i].identity);
    let root_file = format!("{}/{}",
                            config_dir,
                            &config.root_cert);
    let privkey = get_privkey(&config.aggregator[i].privkey);    

    let foll0_addr = format!("{}:{}",
                            config.follower[i].ip,
                            config.follower[i].port1);    
    let foll1_addr = format!("{}:{}",
                            config.follower[i + 2].ip,
                            config.follower[i + 2].port1);        

    println!("Forwarding to: {} and {}", foll0_addr, foll1_addr);

    let stream0 = tls_connect(foll0_addr.clone(),
                              root_file.clone(),
                              identity_file.clone(),
                              config.follower[i].ip.clone(),
                              config.aggregator[i].password.clone()).await?;
    let stream0 = Arc::new(Mutex::new(stream0));
    let stream1 = tls_connect(foll1_addr.clone(),
                              root_file.clone(),
                              identity_file.clone(),
                              config.follower[i + 2].ip.clone(),
                              config.aggregator[i].password.clone()).await?;
    let stream1 = Arc::new(Mutex::new(stream1));    

    loop {
        let (socket, _) = listener.accept().await?;
        let stream0 = Arc::clone(&stream0);
        let stream1 = Arc::clone(&stream1);
        let acceptor = acceptor.clone();
        let privkey = privkey.clone();

        tokio::spawn(async move {
            // Check: Move out of loop?
            let mut stream = acceptor.accept(socket).await.expect("accept error");
            //let mut count = 0u32;
            loop {
                let stream0 = Arc::clone(&stream0);
                let stream1 = Arc::clone(&stream1);                
                let privkey = privkey.clone();
                let mut buf = vec![0; 8192];
                match stream.read(&mut buf).await {
                    Ok(0) => (),
                    Ok(n) => {
                        //count += 1;                                                
                        //println!("{:?}", count);
                        //println!("Received {:?} bytes\n", n);
                        tokio::spawn(async move {
                            let payload = decrypt_to_payloadseed(&buf[0..n],
                                                             &privkey);
                            let payload_bytes = bincode::serialize(&payload).unwrap();
                            if payload.index >= 1000 {
                                let mut stream0 = stream0.lock().await;
                                stream0
                                    .write_all(&payload_bytes)
                                    .await
                                    .expect("Failed to write data to socket");
                            } else {
                                let mut stream1 = stream1.lock().await;
                                stream1
                                    .write_all(&payload_bytes)
                                    .await
                                    .expect("Failed to write data to socket");
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
                       config.aggregator[i].ip,
                       config.aggregator[i].port1);

    println!("Listening on: {}", addr);
    
    let listener = TcpListener::bind(addr).await?;
    let identity_file = format!("{}/{}",
                                config_dir,
                                &config.proxy[i].identity);
    let acceptor = get_acceptor(&identity_file);
    let acceptor = tokio_native_tls::TlsAcceptor::from(acceptor);

    match i {
        0 => aggregator0(i,
                         config_dir,
                         listener,
                         acceptor).await?,
        1 => aggregator1(i,
                         config_dir,
                         listener,
                         acceptor).await?,
        _ => println!("Error!\n"), // TODO: Error
    }
    Ok(())    
}
