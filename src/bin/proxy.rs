use hpke::kex::KeyExchange;
use native_tls::{Certificate, Identity, TlsConnector};
use pa::{config::get_config,
         hpke::{get_privkey, HPKEOut, hpke_decrypt, Kex},
         payload::InnerCiphertexts,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use tokio_native_tls::TlsStream;
use std::{env,
          error::Error,
          fs::File,
          io::Read,
          sync::Arc,
          time::SystemTime,
};

fn decrypt_to_inners(
    contents: &[u8],
    privkey: &<Kex as KeyExchange>::PrivateKey
) -> InnerCiphertexts {
    let hpke_out: HPKEOut = bincode::deserialize(&contents).unwrap();
    let decrypted_msg = hpke_decrypt(
        privkey,
        &hpke_out,
        b"",
    );
    let inners: InnerCiphertexts = bincode::deserialize(&decrypted_msg).unwrap();
    inners
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
                                    "config.toml"))?;
    let identity_file = format!("{}/{}",
                                config_dir,
                                &config.proxy[i].identity);
    let root_file = format!("{}/{}",
                            config_dir,
                            &config.root_cert);
    let privkey = get_privkey(&config.proxy[i].privkey);
    let listen_addr = format!("{}:{}",
                              config.proxy[i].ip,
                              config.proxy[i].port);        
    let agg0_addr = format!("{}:{}",
                            config.aggregator[0].ip,
                            config.aggregator[0].port1);    
    let agg1_addr = format!("{}:{}",
                            config.aggregator[1].ip,
                            config.aggregator[1].port1);        

    println!("Listening on: {}", listen_addr);
    println!("Proxying to: {} and {}", agg0_addr, agg1_addr);  
    
    let listener = TcpListener::bind(listen_addr).await?;

    let stream0 = tls_connect(agg0_addr.clone(),
                              root_file.clone(),
                              identity_file.clone(),
                              config.aggregator[0].ip.clone(),
                              config.proxy[i].password.clone()).await?;
    let stream0 = Arc::new(Mutex::new(stream0));
    let stream1 = tls_connect(agg1_addr.clone(),
                              root_file.clone(),
                              identity_file.clone(),
                              config.aggregator[1].ip.clone(),
                              config.proxy[i].password.clone()).await?;
    let stream1 = Arc::new(Mutex::new(stream1));

    let mut count: u32  = 0;
    loop {
        let (mut inbound, _) = listener.accept().await?;
        let privkey = privkey.clone();
        let stream0 = Arc::clone(&stream0);
        let stream1 = Arc::clone(&stream1);
        count += 1;
        if count == 1 {
            println!("Start time: {:?}\n", SystemTime::now());
        }
        
        tokio::spawn(async move {
            let privkey = privkey.clone();
            let stream0 = Arc::clone(&stream0);
            let stream1 = Arc::clone(&stream1);
            let mut buf= vec![0; 8192]; // TODO: Update
            match inbound.read(&mut buf).await {
                Ok(0) => println!("Received 0 bytes {:?}\n", count),
                Ok(n) => {
                    //println!("Received {:?} bytes\n", n);
                    let inners = decrypt_to_inners(&buf[0..n], &privkey);
                    let mut stream0 = stream0.lock().await;
                    stream0
                        .write_all(&inners.left)
                        .await
                        .expect("Failed to write data to socket");
                    let mut stream1 = stream1.lock().await;                    
                    stream1
                        .write_all(&inners.right)
                        .await
                        .expect("Failed to write data to socket");                            
                }
                Err(_) => println!("Error\n"),
            };                
        });
    }
}
