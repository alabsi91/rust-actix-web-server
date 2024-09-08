use actix_files::NamedFile;
use actix_web::{web, HttpRequest, Result};
use std::path::PathBuf;

use crate::config::CONFIG;

pub async fn file_handler((req, path): (HttpRequest, web::Path<String>)) -> Result<NamedFile> {
    let connection_info = req.connection_info();
    let client_ip = connection_info.realip_remote_addr().unwrap_or("unknown ip");

    // first check without the trailing slash, by removing it
    let file_path_no_slash =
        PathBuf::from(&CONFIG.public_dir).join(path.as_str().trim_end_matches('/'));
    if file_path_no_slash.exists() && file_path_no_slash.is_file() {
        println!("[{}] Serving: {}", client_ip, file_path_no_slash.display());
        return Ok(NamedFile::open(file_path_no_slash)?);
    }

    let file_path: PathBuf = PathBuf::from(&CONFIG.public_dir).join(path.as_str());

    // serve assets
    if file_path.exists() && file_path.is_file() {
        println!("[{}] Serving: {}", client_ip, file_path.display());
        return Ok(NamedFile::open(file_path)?);
    }

    // search for index.html file and serve it
    let mut index_dir: PathBuf = PathBuf::from(&CONFIG.public_dir).join(path.as_str());
    while !index_dir.as_os_str().is_empty() {
        let index_path = index_dir.join("index.html");
        if index_path.exists() {
            println!("[{}] Serving: {}", client_ip, index_path.display());
            return Ok(NamedFile::open(index_path)?);
        }
        index_dir.pop();
    }

    // 404 page not found
    println!("[{}] File not found: {}", client_ip, file_path.display());
    return Ok(NamedFile::open(PathBuf::from(&CONFIG.not_found_page))?);
}
