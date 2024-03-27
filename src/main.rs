use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::http::{Request, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use log::{debug, info};
use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use crate::room::{room_thread, FromBrowser, Room, ToBrowser};

mod room;

#[derive(Clone, Default)]
struct AppState {
    rooms: Arc<RwLock<HashMap<String, Arc<Room>>>>,
}

impl AppState {
    async fn find_room(&self, name: &str) -> Option<Arc<Room>> {
        self.rooms.read().await.get(name).cloned()
    }

    async fn add_room(&self, room: Arc<Room>) {
        let mut rooms = self.rooms.write().await;

        rooms.insert(room.name.clone(), room);
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Kolla kolla!");

    let state = AppState::default();
    let (room, room_receiver) = Room::new(
        "ASDF".into(),
        "http://0.0.0.0:8000/WING%20IT%21%20-%20Blender%20Open%20Movie-1080p.mp4".into(),
    );
    let room = Arc::new(room);
    state.add_room(room.clone()).await;
    tokio::spawn(async move { room_thread(room.clone(), room_receiver).await });

    let app = Router::new()
        .route("/api/:room/:name/", get(room_websocket_handler))
        .fallback(get_asset)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8003").await.unwrap();

    info!("Listning on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn default_index_html_response() -> Response<Body> {
    let content = tokio::fs::read("site/index.html").await.unwrap();
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html")
        .body(Body::from(content))
        .unwrap()
}

#[axum::debug_handler]
async fn room_websocket_handler(
    Path((room, name)): Path<(String, String)>,
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> axum::response::Response<Body> {
    debug!("Got WS request for {room:?} with name {name:?}");

    let Some(room) = state.find_room(&room).await else {
        return axum::response::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap();
    };

    let recv = room.add_watcher(name.clone()).await;

    ws.on_upgrade(move |socket| room_websocket(socket, recv, room, name))
}

async fn room_websocket(
    socket: WebSocket,
    mut room_recv: Receiver<ToBrowser>,
    room: Arc<Room>,
    name: String,
) {
    let (mut write, mut read) = socket.split();

    tokio::select! {
        Some(msg) = read.next() => {
            debug!("{}/{}: from browser: {:?}", room.name, name, msg);

            let msg = if let Ok(msg) = msg {
                msg
            } else {
                // client disconnected
                return;
            };

            let msg = parse_msg(msg).unwrap();

            room.send(msg).await;
        }
        Some(msg) = room_recv.recv() => {
            debug!("{}/{}: to browser: {:?}", room.name, name, msg);

            write.send(Message::Text(serde_json::to_string(&msg).unwrap())).await.unwrap();
        }
    }
}

fn parse_msg(msg: Message) -> anyhow::Result<FromBrowser> {
    match msg {
        Message::Text(t) => Ok(serde_json::from_str(&t)?),
        _ => anyhow::bail!("invalid message type"),
    }
}

async fn get_asset(uri: Uri) -> Result<Response<Body>, (StatusCode, String)> {
    debug!("Fallback for request {uri:?}");

    if uri.path().starts_with("/api") {
        return Err((StatusCode::NOT_FOUND, "Not Found".to_string()));
    }

    if uri.path() == "/" {
        return Ok(default_index_html_response().await.into_response());
    }

    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    match ServeDir::new("site").try_call(req).await {
        Ok(res) => {
            if res.status() != StatusCode::OK {
                Ok(default_index_html_response().await.into_response())
            } else {
                Ok(res.into_response())
            }
        }
        Err(_err) => Ok(default_index_html_response().await.into_response()),
    }
}

/*fn main2() {
    env_logger::init();

    info!("Kolla kolla!");

    let mut state = Arc::new(State::default());
    state.add_room(Room::new(
        "test-room".into(),
        "http://0.0.0.0:8000/WING%20IT%21%20-%20Blender%20Open%20Movie-1080p.mp4".into(),
    ));

    let server = tiny_http::Server::http("127.0.0.1:8003").unwrap();

    info!("Listening for HTTP requests...");
    for req in server.incoming_requests() {
        thread::spawn(move || {
            let method = req.method().clone();
            let url = req.url().to_string();

            debug!("Got {method} {url}");

            if let Err(e) = process(req, state.clone()) {
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

fn process(req: Request, state: Arc<State>) -> anyhow::Result<()> {
    let url = req.url();

    // room websocket
    if url.starts_with("/api/room/") {
        let room = &url[10..];

        let Some(room) = state.find_room(room) else {
            req.respond(Response::from_string("").with_status_code(404))?;
            return Ok(());
        };

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
        room::process_room(websocket, room)?;

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
}*/
