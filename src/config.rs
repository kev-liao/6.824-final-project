use crate::flpcp::QueryState;
use rug::Integer;
use serde::{Serialize, Deserialize};
use std::{str,
          fs::File,
          io::Read};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub root_cert: String,
    pub queries: String,
    pub input_len: u64,
    pub prime: Integer,
    pub generator: Integer,
    pub proxy: Vec<Proxy>,
    pub aggregator: Vec<Aggregator>,
    pub follower: Vec<Follower>,    
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Proxy {
    pub ip: String,
    pub port: String,
    pub pubkey: String,
    pub privkey: String,
    pub identity: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Aggregator {
    pub seed: u64,
    pub ip: String,
    pub port1: String,
    pub port2: String,        
    pub pubkey: String,
    pub privkey: String,
    pub identity: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Follower {
    pub seed: u64,
    pub ip: String,
    pub port1: String,
    pub port2: String,        
    pub pubkey: String,
    pub privkey: String,
    pub identity: String,
    pub password: String,
}

pub fn get_config(filename: String) -> std::io::Result<Config> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: Config = toml::from_str(&contents).unwrap();
    Ok(config)
}

pub fn get_queries(filename: String) -> std::io::Result<QueryState> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let precompute: QueryState = toml::from_str(&contents).unwrap();
    Ok(precompute)
}
