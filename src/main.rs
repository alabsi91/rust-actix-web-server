use actix_files::{Directory, NamedFile};
use actix_web::dev::ServiceResponse;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use askama_escape::{escape as escape_html_entity, Html};
use once_cell::sync::Lazy;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use percent_encoding::{utf8_percent_encode, CONTROLS};
use serde::Deserialize;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::PathBuf;
use std::{fmt::Write, path::Path};

#[derive(Debug, Deserialize, Clone)]
struct Config {
    https: bool,
    https_port: u16,
    openssl_key: String,
    openssl_cert: String,
    ip: String,
    port: u16,
    not_found_file: String,
    serve_dir: String,
    file_listing_dir: String,
    file_listing_entry: String,
}

fn read_config_file(filename: &str) -> Config {
    // Read the JSON file
    let file = File::open(filename).expect("Failed to open config.json file");
    let reader = BufReader::new(file);

    // Deserialize JSON into Config struct
    serde_json::from_reader(reader).expect("Failed to parse config.json file")
}

static CONFIG: Lazy<Config> = Lazy::new(|| read_config_file("config.json"));

#[get("/{sub_path}")]
async fn index_without_slash(path: web::Path<String>) -> HttpResponse {
    let sub_path = path.into_inner();
    HttpResponse::Found()
        .append_header(("Location", sub_path + "/"))
        .finish()
}

async fn file_handler((_req, path): (HttpRequest, web::Path<String>)) -> Result<NamedFile> {
    let file_path: PathBuf = PathBuf::from(&CONFIG.serve_dir).join(path.as_str());

    // serve assets
    if file_path.exists() && file_path.is_file() {
        println!("Serving: {}", file_path.display());
        return Ok(NamedFile::open(file_path)?);
    }

    // search for index.html file and serve it
    let mut index_dir: PathBuf = PathBuf::from(&CONFIG.serve_dir).join(path.as_str());
    while !index_dir.as_os_str().is_empty() {
        let index_path = index_dir.join("index.html");
        if index_path.exists() {
            return Ok(NamedFile::open(index_path)?);
        }
        index_dir.pop();
    }

    // 404 page not found
    println!("File not found: {}", file_path.display());
    return Ok(NamedFile::open(PathBuf::from(&CONFIG.not_found_file))?);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let ip_address = CONFIG.ip.clone();

    let server = || {
        App::new()
            .service(index_without_slash)
            .service(
                actix_files::Files::new(&CONFIG.file_listing_entry, &CONFIG.file_listing_dir)
                    .show_files_listing()
                    .files_listing_renderer(directory_listing),
            )
            .service(web::resource("/{path:.*}").route(web::get().to(file_handler)))
    };

    let http_server = HttpServer::new(server).bind((ip_address.clone(), CONFIG.port))?;

    if CONFIG.https {
        println!(
            "Starting server on https://{}:{} and http://{}:{}",
            ip_address.clone(),
            CONFIG.https_port,
            ip_address.clone(),
            CONFIG.port
        );
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder
            .set_private_key_file(CONFIG.openssl_key.clone(), SslFiletype::PEM)
            .unwrap();
        builder
            .set_certificate_chain_file(CONFIG.openssl_cert.clone())
            .unwrap();

        let https_server = HttpServer::new(server)
            .bind_openssl((ip_address.clone(), CONFIG.https_port), builder)?;

        tokio::try_join!(https_server.run(), http_server.run())?;

        Ok(())
    } else {
        println!("Starting server on http://{}:{}", ip_address, CONFIG.port);
        tokio::try_join!(http_server.run())?;

        Ok(())
    }
}

// Returns percent encoded file URL path.
macro_rules! encode_file_url {
    ($path:ident) => {
        utf8_percent_encode(&$path, CONTROLS)
    };
}

macro_rules! encode_file_name {
    ($entry:ident) => {
        escape_html_entity(&$entry.file_name().to_string_lossy(), Html)
    };
}

fn directory_listing(dir: &Directory, req: &HttpRequest) -> Result<ServiceResponse, io::Error> {
    let index_of = format!("📂 {}", req.path());
    let mut body = String::new();
    let base = Path::new(req.path());

    for entry in dir.path.read_dir()? {
        if dir.is_visible(&entry) {
            let entry = entry.unwrap();
            let p = match entry.path().strip_prefix(&dir.path) {
                Ok(p) if cfg!(windows) => base.join(p).to_string_lossy().replace('\\', "/"),
                Ok(p) => base.join(p).to_string_lossy().into_owned(),
                Err(_) => continue,
            };

            // if file is a directory, add '/' to the end of the name
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    let _ = write!(
                        body,
                        "<li><a href=\"{}\">{}/</a></li>",
                        encode_file_url!(p),
                        encode_file_name!(entry),
                    );
                } else {
                    let _ = write!(
                        body,
                        "<li style='list-style: &quot📄&quot;'><a href=\"{}\">{}</a></li>",
                        encode_file_url!(p),
                        encode_file_name!(entry),
                    );
                }
            } else {
                continue;
            }
        }
    }

    let html = format!(
        "<html>\
         <head><title>{}</title></head>\
         <body>\
         <style>\
            body {{\
                background-color: #202124;\
                font-family: monospace;\
            }}\
            h1 {{\
                color: #93b5f6;\
                margin: 50px 10px;\
            }}\
            a {{\
                all: unset;\
                margin-left: 10px;\
                color: #aaa;\
            }}\
            a:hover {{\
                text-decoration: underline;\
                color: white;\
            }}\
            ul {{\
                color: #93b5f6;\
            }}\
            li {{\
                margin: 15px 0;\
                font-size: 20px;\
                list-style: '📁';\
                cursor: pointer;\
            }}\
         </style>\
         <h1>{}</h1>\
         <ul>\
         <li style='list-style:\"↩️\"'><a href='javascript:history.back()'>Go Back</a></li>\
         {}\
         </ul></body>\n</html>",
        index_of, index_of, body
    );
    Ok(ServiceResponse::new(
        req.clone(),
        HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html),
    ))
}
