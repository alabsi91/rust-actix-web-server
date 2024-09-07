use actix_files::Directory;
use actix_web::dev::ServiceResponse;
use actix_web::{HttpRequest, HttpResponse, Result};
use askama_escape::{escape as escape_html_entity, Html};
use percent_encoding::{utf8_percent_encode, CONTROLS};
use std::io;
use std::{fmt::Write, path::Path};

pub fn directory_listing(dir: &Directory, req: &HttpRequest) -> Result<ServiceResponse, io::Error> {
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

    let index_of = format!("📂 {}", req.path());
    let mut body = String::new();
    let base = Path::new(req.path());

    let connection_info = req.connection_info();
    let client_ip = connection_info.realip_remote_addr().unwrap_or("unknown ip");

    println!("[{}] Serving: {}", client_ip, dir.path.display());

    for entry in dir.path.read_dir()? {
        if !dir.is_visible(&entry) {
            continue;
        }

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

                continue;
            }

            let _ = write!(
                body,
                "<li style='list-style: &quot📄&quot;'><a href=\"{}\">{}</a></li>",
                encode_file_url!(p),
                encode_file_name!(entry),
            );
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
