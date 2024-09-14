mod config;
mod files_listing;
mod serve;
mod utils;

use actix_governor::{Governor, GovernorConfigBuilder};
use actix_ip_filter::IPFilter;
use actix_web::{
    dev::{self, Server},
    get, web, App, HttpResponse, HttpServer,
};
use colored::Colorize;
use futures_util::future::try_join_all;
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslFiletype, SslMethod};

use config::CONFIG;
use files_listing::directory_listing;
use serve::file_handler;

#[get("/{sub_path}")]
async fn index_without_slash(path: web::Path<String>) -> HttpResponse {
    let sub_path = path.into_inner();
    HttpResponse::Found()
        .append_header(("Location", sub_path + "/"))
        .finish()
}

fn on_block(_flt: &IPFilter, ip: &str, req: &dev::ServiceRequest) -> Option<HttpResponse> {
    println!(
        "[{}] {} {}",
        ip.to_string().blue(),
        "Blocked:".red(),
        req.path().to_string().yellow()
    );
    return Some(HttpResponse::Forbidden().body("Forbidden"));
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let governor_res = GovernorConfigBuilder::default()
        .per_second(CONFIG.filtering.rate_limit.per_second)
        .burst_size(CONFIG.filtering.rate_limit.burst_size)
        .finish();

    let governor_conf;
    match governor_res {
        Some(conf) => governor_conf = conf,
        None => {
            println!("{}", "Error: Failed to create governor configuration".red());
            std::process::exit(1);
        }
    }

    let base_server = move || {
        let blacklist = CONFIG
            .filtering
            .ip_blacklist
            .iter()
            .map(|s| s.as_str())
            .collect();

        let whitelist = CONFIG
            .filtering
            .ip_whitelist
            .iter()
            .map(|s| s.as_str())
            .collect();

        let mut app = App::new().wrap(Governor::new(&governor_conf)).wrap(
            IPFilter::new()
                .allow(whitelist)
                .block(blacklist)
                .on_block(on_block)
                .x_real_ip(true),
        );

        if CONFIG.file_listing.enabled {
            app = app.service(
                actix_files::Files::new(&CONFIG.file_listing.route, &CONFIG.file_listing.dir)
                    .show_files_listing()
                    .files_listing_renderer(directory_listing),
            );
        }

        app.service(index_without_slash)
            .service(web::resource("/{path:.*}").route(web::get().to(file_handler)))
    };

    let mut servers: Vec<Server> = vec![];

    if CONFIG.https.enabled {
        println!(
            "Starting {} server on {}{}{}{}",
            "HTTPS".yellow(),
            "https://".green(),
            CONFIG.https.ip.clone().green(),
            ":".yellow(),
            CONFIG.https.port.to_string().blue()
        );

        let mut builder: SslAcceptorBuilder;
        match SslAcceptor::mozilla_intermediate(SslMethod::tls()) {
            Ok(b) => builder = b,
            Err(e) => {
                println!("{}", "Error: Failed to create SSL Acceptor".red());
                eprintln!("{}", e.to_string().red());
                std::process::exit(1);
            }
        }

        if let Err(e) = builder.set_private_key_file(CONFIG.https.key.clone(), SslFiletype::PEM) {
            println!("{}", "Error: Failed to set SSL private key".red());
            eprintln!("{}", e.to_string().red());
            std::process::exit(1);
        }

        if let Err(e) = builder.set_certificate_chain_file(CONFIG.https.cert.clone()) {
            println!("{}", "Error: Failed to set SSL certificate".red());
            eprintln!("{}", e.to_string().red());
            std::process::exit(1);
        }

        let https_server = HttpServer::new(base_server.clone())
            .bind_openssl((CONFIG.https.ip.clone(), CONFIG.https.port), builder);

        match https_server {
            Ok(server) => servers.push(server.run()),
            Err(e) => {
                println!(
                    "{}",
                    "Error: Failed to bind the server to openSSL HTTPS".red()
                );
                eprintln!("{}", e.to_string().red());
                std::process::exit(1);
            }
        }
    }

    if CONFIG.http.enabled {
        println!(
            "Starting {} server on {}{}{}{}",
            "HTTP".yellow(),
            "http://".green(),
            CONFIG.http.ip.clone().green(),
            ":".yellow(),
            CONFIG.http.port.to_string().blue()
        );

        let http_server =
            HttpServer::new(base_server).bind((CONFIG.http.ip.clone(), CONFIG.http.port));

        match http_server {
            Ok(server) => servers.push(server.run()),
            Err(e) => {
                println!("{}", "Error: Failed to bind the server to HTTP".red());
                eprintln!("{}", e.to_string().red());
                std::process::exit(1);
            }
        }
    }

    if servers.is_empty() {
        println!("No server started");
        return Ok(());
    }

    if let Err(e) = try_join_all(servers).await {
        println!("{}", "Error: Failed to start".red());
        eprintln!("{}", e.to_string().red());
        std::process::exit(1);
    }

    Ok(())
}
