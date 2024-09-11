use actix_files::NamedFile;
use actix_web::{web, HttpRequest, Result};
use colored::Colorize;
use std::path::PathBuf;

use crate::{config::CONFIG, utils::has_traversal};

pub async fn file_handler((req, path): (HttpRequest, web::Path<String>)) -> Result<NamedFile> {
    let connection_info = req.connection_info();
    let client_ip = connection_info.realip_remote_addr().unwrap_or("unknown ip");

    let route_path = PathBuf::from(path.as_str());
    if has_traversal(&route_path) {
        println!(
            "[{}] {} {}",
            client_ip.to_string().blue(),
            "Not Safe:".red(),
            route_path.display().to_string().yellow()
        );
        return Ok(NamedFile::open(PathBuf::from(&CONFIG.not_found_page))?);
    }

    // first check without the trailing slash, by removing it
    let file_path_no_slash =
        PathBuf::from(&CONFIG.public_dir).join(path.as_str().trim_end_matches('/'));

    if file_path_no_slash.exists()
        && file_path_no_slash.is_file()
        && !has_traversal(&file_path_no_slash)
    {
        println!(
            "[{}] Serving: {}",
            client_ip.to_string().blue(),
            file_path_no_slash.display().to_string().yellow()
        );
        return Ok(NamedFile::open(file_path_no_slash)?);
    }

    let file_path: PathBuf = PathBuf::from(&CONFIG.public_dir).join(route_path);

    // serve assets
    if file_path.exists() && file_path.is_file() {
        println!(
            "[{}] Serving: {}",
            client_ip.to_string().blue(),
            file_path.display().to_string().yellow()
        );
        return Ok(NamedFile::open(file_path)?);
    }

    // search for index.html file and serve it
    let mut index_dir = file_path.clone();
    while !index_dir.as_os_str().is_empty() {
        let index_path = index_dir.join("index.html");
        if index_path.exists() {
            println!(
                "[{}] Serving: {}",
                client_ip.to_string().blue(),
                index_path.display().to_string().yellow()
            );
            return Ok(NamedFile::open(index_path)?);
        }
        index_dir.pop();
    }

    // 404 page not found
    println!(
        "[{}] {} {}",
        client_ip.to_string().blue(),
        "File not found:".red(),
        file_path.display().to_string().blue()
    );
    return Ok(NamedFile::open(PathBuf::from(&CONFIG.not_found_page))?);
}
