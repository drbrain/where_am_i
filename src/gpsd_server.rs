mod parser;
mod codec;

use parser::Command;
use codec::Codec;

use crate::JsonSender;

use futures::SinkExt;

use serde_json::json;

use tokio::net::TcpListener;
use tokio::stream::StreamExt;

use tokio_util::codec::Framed;

use tracing::debug;
use tracing::error;
use tracing::info;

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
            Err(_) => Command::Error("unrecognized command".to_string()),
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
                "message": e,
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
