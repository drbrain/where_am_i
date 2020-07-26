mod parser;
mod codec;

use parser::Command;
use codec::Codec;

use crate::JsonSender;

use futures::SinkExt;

use serde_json::json;

use std::cell::RefCell;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::stream::StreamExt;

use tokio_util::codec::Framed;

use tracing::debug;
use tracing::error;
use tracing::info;

#[derive(Clone)]
struct Client {
    json: bool,
    pps: bool,
    watch: bool,
}

impl Client {
    fn new() -> Client {
        Client {
            json: false,
            pps: false,
            watch: false,
        }
    }
}

#[derive(Clone)]
struct Clients(Arc<RefCell<HashMap<SocketAddr, Client>>>);

impl Clients {
    fn new() -> Clients {
        Clients(Arc::new(RefCell::new(HashMap::new())))
    }

    fn add(&self, addr: SocketAddr, client: Client) {
        self.0.borrow_mut().insert(addr, client);
    }

    fn remove(&self, addr: &SocketAddr) -> Option<Client> {
        self.0.borrow_mut().remove(addr)
    }
}

#[tracing::instrument]
pub async fn spawn(port: u16, tx: JsonSender) {
    let address = ("0.0.0.0", port);

    let mut listener = TcpListener::bind(address).await.unwrap();
    let mut incoming = listener.incoming();

    while let Some(socket) = incoming.next().await {
        let socket = match socket {
            Ok(s) => {
                info!("client connected {:?}", s.peer_addr());
                s
            },
            Err(e) => {
                error!("connect failed {:?}", e);
                break;
            },
        };

        let mut gpsd = Framed::new(socket, Codec::new());
        debug!("{:?}", gpsd);

        let result = match gpsd.next().await {
            Some(r) => r,
            None => break,
        };

        let command = match result {
            Ok(c) => c,
            Err(e) => Command::Error("unrecognized command".to_string()),
        };

        let response = match command {
            Command::Devices => json!({
                "class": "DEVICES",
                "devices": [],
            }),
            Command::Device(_) => json!({
                "class": "DEVICE",
                "stopbits": 1,
            }),
            Command::Error(e) => json!({
                "class": "ERROR",
                "message": "unrecognized command",
            }),
            Command::Poll => json!({
                "class": "POLL",
                "time": 0,
                "active": 0,
                "tpv": [],
                "sky": [],
            }),
            Command::Version => json!( {
                "class": "VERSION",
                "release": "",
                "rev": "",
                "proto_major": 3,
                "proto_minor": 10,
            }),
            Command::Watch(_) => json!({
                "class": "WATCH",
            }),
        };

        debug!("{:?}", response);
        gpsd.send(response).await.unwrap();
    }
}
