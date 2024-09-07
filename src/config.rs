use once_cell::sync::Lazy;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub https: Https,
    pub http: Http,
    pub file_listing: FileListing,
    pub not_found_page: String,
    pub public_dir: String,
    pub filtering: Filtering,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Filtering {
    pub ip_blacklist: Vec<String>,
    pub ip_whitelist: Vec<String>,
    pub rate_limit: RateLimit,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimit {
    pub per_second: u64,
    pub burst_size: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Https {
    pub enabled: bool,
    pub ip: String,
    pub port: u16,
    pub key: String,
    pub cert: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Http {
    pub enabled: bool,
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FileListing {
    pub enabled: bool,
    pub dir: String,
    pub route: String,
}

fn read_config_file(filename: &str) -> Config {
    // Read the JSON file
    let file = File::open(filename).expect("Failed to open config.json file");
    let reader = BufReader::new(file);

    // Deserialize JSON into Config struct
    serde_json::from_reader(reader).expect("Failed to parse config.json file")
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| read_config_file("config.json"));
