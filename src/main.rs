use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use log::{debug, info, warn};
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

    let room_name = env::var("ROOM_NAME").unwrap();
    let room_url = env::var("ROOM_URL").unwrap();
    let serve_dir = env::var("SERVE_DIR").unwrap();

    let state = AppState::default();
    let (room, room_receiver) = Room::new(room_name, room_url);
    let room = Arc::new(room);
    state.add_room(room.clone()).await;
    tokio::spawn(async move { room_thread(room.clone(), room_receiver).await });

    let app = Router::new()
        .route("/api/:room/:name/", get(room_websocket_handler))
        .nest_service("/", ServeDir::new(serve_dir))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8003").await.unwrap();

    info!("Listning on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
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

    let (recv, id) = room.add_watcher(name.clone()).await;

    ws.on_upgrade(move |socket| async move {
        if let Err(e) = room_websocket(socket, recv, room.clone(), name.clone(), id).await {
            warn!("{e}");
        }

        debug!("Removing watcher {name} ({id})");
        room.remove_watcher(id).await;
    })
}

async fn room_websocket(
    socket: WebSocket,
    mut room_recv: Receiver<ToBrowser>,
    room: Arc<Room>,
    name: String,
    id: u32,
) -> anyhow::Result<()> {
    let (mut write, mut read) = socket.split();
    write
        .send(Message::Text(
            serde_json::to_string(&ToBrowser::Id(id)).unwrap(),
        ))
        .await?;

    loop {
        tokio::select! {
            Some(msg) = read.next() => {
                debug!("{}/{}: from browser: {:?}", room.name, name, msg);

                let msg = msg?;
                let msg = parse_msg(msg, id)?;

                room.send(msg).await;
            }
            Some(msg) = room_recv.recv() => {
                debug!("{}/{}: to browser: {:?}", room.name, name, msg);

                write.send(Message::Text(serde_json::to_string(&msg).unwrap())).await?;
                write.flush().await?;
            }
        }
    }

    Ok(())
}

fn parse_msg(msg: Message, id: u32) -> anyhow::Result<FromBrowser> {
    match msg {
        Message::Text(t) => {
            let msg = serde_json::from_str(&t)?;
            let msg = match msg {
                FromBrowser::Play {
                    request_id, time, ..
                } => FromBrowser::Play {
                    id: Some(id),
                    request_id,
                    time,
                },
                FromBrowser::Pause {
                    request_id, time, ..
                } => FromBrowser::Pause {
                    id: Some(id),
                    request_id,
                    time,
                },
                FromBrowser::Seek {
                    request_id, time, ..
                } => FromBrowser::Seek {
                    id: Some(id),
                    request_id,
                    time,
                },
                FromBrowser::Status {
                    position,
                    buffered,
                    state,
                    ..
                } => FromBrowser::Status {
                    id: Some(id),
                    position,
                    buffered,
                    state,
                },
                _ => msg,
            };
            Ok(msg)
        }
        _ => anyhow::bail!("invalid message type"),
    }
}
