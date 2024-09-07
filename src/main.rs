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
use futures_util::future::try_join_all;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

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

fn on_block(_flt: &IPFilter, ip: &str, _req: &dev::ServiceRequest) -> Option<HttpResponse> {
    println!("[{}] Blocked", ip);
    return Some(HttpResponse::Forbidden().body("Forbidden"));
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(CONFIG.filtering.rate_limit.per_second)
        .burst_size(CONFIG.filtering.rate_limit.burst_size)
        .finish()
        .unwrap();

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
                .on_block(on_block),
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
            "Starting HTTPS server on https://{}:{}",
            CONFIG.https.ip.clone(),
            CONFIG.https.port,
        );

        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder
            .set_private_key_file(CONFIG.https.key.clone(), SslFiletype::PEM)
            .unwrap();
        builder
            .set_certificate_chain_file(CONFIG.https.cert.clone())
            .unwrap();

        let https_server = HttpServer::new(base_server.clone())
            .bind_openssl((CONFIG.https.ip.clone(), CONFIG.https.port), builder)
            .unwrap()
            .run();

        servers.push(https_server);
    }

    if CONFIG.http.enabled {
        println!(
            "Starting HTTP server on http://{}:{}",
            CONFIG.http.ip.clone(),
            CONFIG.http.port
        );

        let http_server = HttpServer::new(base_server)
            .bind((CONFIG.http.ip.clone(), CONFIG.http.port))
            .unwrap()
            .run();

        servers.push(http_server);
    }

    if servers.is_empty() {
        println!("No server started");
        return Ok(());
    }

    try_join_all(servers).await?;

    Ok(())
}
