use std::path::{Component, Path, PathBuf};
use std::{env, fs, thread};

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use tiny_http::{Request, Response, ResponseBox};
use tungstenite::protocol::{Role, WebSocket};

fn main() {
    env_logger::init();

    info!("Kolla kolla!");

    let server = tiny_http::Server::http("127.0.0.1:8003").unwrap();

    info!("Listening for HTTP requests...");
    for req in server.incoming_requests() {
        thread::spawn(move || {
            let method = req.method().clone();
            let url = req.url().to_string();

            debug!("Got {method} {url}");

            if let Err(e) = process(req) {
                warn!("Error processing {method} {url}: {e}");
            }
        });
    }
}

fn convert_key(input: &str) -> String {
    use base64::{engine::general_purpose::STANDARD, engine::Engine};
    use sha1::{Digest, Sha1};

    let mut input = input.to_string().into_bytes();
    let mut bytes = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
        .to_string()
        .into_bytes();
    input.append(&mut bytes);

    let mut sha1 = Sha1::new();
    sha1.update(&input);

    STANDARD.encode(sha1.finalize())
}

fn process(req: Request) -> anyhow::Result<()> {
    let url = req.url();

    // room websocket
    if url.starts_with("/api/room/") {
        let room = &url[10..];

        dbg!(room);

        let key = match req
            .headers()
            .iter()
            .find(|h| h.field.equiv(&"Sec-WebSocket-Key"))
            .map(|h| h.value.clone())
        {
            None => {
                req.respond(Response::new_empty(tiny_http::StatusCode(400)))?;
                return Ok(());
            }
            Some(k) => k,
        };

        let response = tiny_http::Response::new_empty(tiny_http::StatusCode(101))
            .with_header("Upgrade: websocket".parse::<tiny_http::Header>().unwrap())
            .with_header("Connection: Upgrade".parse::<tiny_http::Header>().unwrap())
            .with_header(
                "Sec-WebSocket-Protocol: ping"
                    .parse::<tiny_http::Header>()
                    .unwrap(),
            )
            .with_header(
                format!("Sec-WebSocket-Accept: {}", convert_key(key.as_str()))
                    .parse::<tiny_http::Header>()
                    .unwrap(),
            );

        let stream = req.upgrade("websocket", response);

        let mut websocket = WebSocket::from_raw_socket(stream, Role::Server, None);
        while let Ok(msg) = websocket.read() {
            dbg!(msg);
        }

        return Ok(());
    }

    // static files
    if url.starts_with("/static/") {
        let file = PathBuf::from(format!("./site/{}", &url[8..]));

        if !file.components().any(|c| c == Component::ParentDir) {
            if let Ok(bytes) = fs::read(file) {
                req.respond(Response::from_data(bytes).with_status_code(200))?;
                return Ok(());
            } else {
                req.respond(Response::from_string("").with_status_code(404))?;
                return Ok(());
            }
        } else {
            req.respond(Response::from_string("").with_status_code(404))?;
            return Ok(());
        }
    }

    // favicon
    if url.ends_with("favicon.ico") {
        req.respond(Response::from_data(include_bytes!("../favicon.ico")).with_status_code(200))?;
        return Ok(());
    }

    // fallback to SPA
    let content = fs::read("site/index.html").unwrap();
    req.respond(Response::from_data(content).with_status_code(200))?;

    return Ok(());
}
