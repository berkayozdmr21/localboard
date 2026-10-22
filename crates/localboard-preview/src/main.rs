//! Optional loopback companion for browser QA; the desktop build uses native IPC.
use localboard_core::Board;
use serde_json::{json, Value};
use std::{
    fs,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tiny_http::{Header, Method, Response, Server};
fn main() {
    let root = std::env::var("LOCALBOARD_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap());
    let data = std::env::var("LOCALBOARD_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join(".localboard"));
    let board = Arc::new(Mutex::new(Board::open(data).expect("Library storage")));
    let port = std::env::var("LOCALBOARD_PORT").unwrap_or("9140".into());
    let host = format!("127.0.0.1:{port}");
    let server = Server::http(&host).expect("Port unavailable");
    println!("LocalBoard: http://{host}");
    for mut req in server.incoming_requests() {
        let board = board.clone();
        let root = root.clone();
        let host = host.clone();
        std::thread::spawn(move || {
            let header = |name: &str| {
                req.headers()
                    .iter()
                    .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
                    .map(|h| h.value.as_str().to_string())
            };
            if header("Host").as_deref() != Some(&host) {
                let _ = req.respond(Response::from_string("Invalid host").with_status_code(403));
                return;
            }
            if req.url() == "/api/dispatch" && req.method() == &Method::Post {
                if header("X-Localboard-Client").as_deref() != Some("studio")
                    || header("Origin").is_some_and(|o| o != format!("http://{host}"))
                {
                    let _ =
                        req.respond(Response::from_string("Invalid source").with_status_code(403));
                    return;
                }
                let mut body = String::new();
                let read = req.as_reader().take(65537).read_to_string(&mut body);
                let result: std::result::Result<Value, String> =
                    if read.is_err() || body.len() > 65536 {
                        Err("İstek boyutu geçersiz.".into())
                    } else {
                        serde_json::from_str::<Value>(&body)
                            .map_err(|_| "Geçersiz JSON.".into())
                            .and_then(|v| {
                                board
                                    .lock()
                                    .map_err(|_| "Kütüphane kilidi alınamadı.".to_string())?
                                    .dispatch(v["action"].as_str().unwrap_or(""), v["args"].clone())
                            })
                    };
                let (status, value) = match result {
                    Ok(value) => (200, json!({"data":value})),
                    Err(error) => (400, json!({"error":error})),
                };
                let _ = req.respond(
                    Response::from_string(value.to_string())
                        .with_status_code(status)
                        .with_header(
                            Header::from_bytes("Content-Type", "application/json; charset=utf-8")
                                .unwrap(),
                        ),
                );
            } else if req.method() == &Method::Get {
                let url = req.url().split('?').next().unwrap_or("/");
                let relative = if url == "/" {
                    "index.html"
                } else {
                    url.trim_start_matches('/')
                };
                if relative.contains("..") || relative.contains('\\') || relative.contains(':') {
                    let _ = req.respond(Response::from_string("Not found").with_status_code(404));
                    return;
                }
                let file = root.join("dist").join(relative);
                match fs::read(&file) {
                    Ok(data) => {
                        let mime = match file.extension().and_then(|x| x.to_str()) {
                            Some("js") => "text/javascript",
                            Some("css") => "text/css",
                            Some("svg") => "image/svg+xml",
                            _ => "text/html; charset=utf-8",
                        };
                        let _ = req.respond(
                            Response::from_data(data)
                                .with_header(Header::from_bytes("Content-Type", mime).unwrap()),
                        );
                    }
                    Err(_) => {
                        let _ =
                            req.respond(Response::from_string("Not found").with_status_code(404));
                    }
                }
            } else {
                let _ = req.respond(Response::from_string("Not found").with_status_code(404));
            }
        });
    }
}
