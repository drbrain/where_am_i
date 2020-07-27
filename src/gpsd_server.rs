mod parser;
mod codec;

use parser::Command;
use codec::Codec;
use codec::CodecError;

use crate::JsonSender;
use crate::JsonReceiver;

use futures::SinkExt;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio::sync::Mutex;

use tokio_util::codec::Framed;

use tracing::debug;
use tracing::error;
use tracing::info;

#[tracing::instrument]
pub async fn run(port: u16, tx: JsonSender) -> Result<(), Box<dyn Error>> {
    let server = Arc::new(Mutex::new(GpsdServer::new(tx)));
    let address = ("0.0.0.0", port);

    let mut listener = TcpListener::bind(address).await?;
    info!("listening on {} port {}", listener.local_addr()?.ip(), port);

    loop {
        let (stream, addr) = listener.accept().await?;

        let server = Arc::clone(&server);

        tokio::spawn(async move {
            match client(server, stream, addr).await {
                Ok(_) => debug!("client {:?} disconnected", addr),
                Err(e) => error!("client {:?} errored: {:?}", addr, e),
            }
        });
    }
}

struct GpsdServer {
    clients: HashMap<SocketAddr, ()>,
    tx: JsonSender,
    watch: Watch,
}

impl GpsdServer {
    fn new(tx: JsonSender) -> Self {
        GpsdServer {
            clients: HashMap::new(),
            tx: tx,
            watch: Watch { class: "WATCH".to_string(), ..Default::default() },
        }
    }

    async fn send(&mut self, message: Value) {
        self.tx.send(message);
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
struct Watch {
    class: String,
    enable: bool,
    json: bool,
    nmea: bool,
    raw: u64,
    scaled: bool,
    split24: bool,
    pps: bool,
    device: Option<String>,
    remote: Option<String>,
}

struct Client {
    client: Framed<TcpStream, Codec>,
    tx: JsonSender,
}

impl Client {
    async fn new(server: Arc<Mutex<GpsdServer>>, client: Framed<TcpStream, Codec>) -> io::Result<Client> {
        let addr = client.get_ref().peer_addr()?;

        let mut server = server.lock().await;

        server.clients.insert(addr, ());

        let tx = server.tx.clone();

        Ok(Client { client, tx })
    }
}

// impl Stream for Client {
//     type Item = Result<Value, CodecError>;
// 
//     fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context <'_>) -> Poll<Option<Self::Item>> {
//         if let Poll::Ready(Some(v)) = Pin::new(&mut self.rx).poll_next(cx) {
//             return Poll::Ready(Some(Ok(v)));
//         }
// 
//         let result: Option<_> = futures::ready!(Pin::new(&mut self.client).poll_next(cx));
// 
//         Poll::Ready(match result {
//             Some(Ok(command)) => Some(Ok(command)),
//             Some(Err(e)) => Some(Err(e)),
//             None => None,
//         })
//     }
// }

async fn client(server: Arc<Mutex<GpsdServer>>, stream: TcpStream, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut client = Framed::new(stream, Codec::new());

    while let Some(result) = client.next().await {
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
            Command::Version => json!({
                "class": "VERSION",
                "release": "",
                "rev": "",
                "proto_major": 3,
                "proto_minor": 10,
            }),
            Command::Watch(w) => command_watch(server.clone(), w).await,
        };

        debug!("{:?}", response);
        client.send(response).await?;
    }

    {
        let mut server = server.lock().await;
        server.clients.remove(&addr);
    }

    Ok(())
}

async fn command_watch(server: Arc<Mutex<GpsdServer>>, updates: Option<parser::WatchData>) -> Value {
    let mut server = server.lock().await;

    match updates {
        Some(u) => {
            server.watch.enable = u.enable;
            server.watch.json = u.json;
            server.watch.nmea = u.nmea;
            server.watch.raw = u.raw;
            server.watch.scaled = u.scaled;
            server.watch.split24 = u.split24;
            server.watch.pps = u.pps;
            server.watch.device = u.device;
            server.watch.remote = None;
        },
        None => (),
    };

    match serde_json::to_value(&server.watch) {
        Ok(w) => w,
        Err(_) => json!({
            "class": "ERROR",
            "message": "internal error",
        }),
    }
}
