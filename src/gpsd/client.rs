use crate::gpsd::codec::Codec;
use crate::gpsd::parser::Command;
use crate::gpsd::server::Server;
use crate::gpsd::Device;
use crate::gpsd::Devices;
use crate::gpsd::ErrorMessage;
use crate::gpsd::Poll;
use crate::gpsd::Response;
use crate::gpsd::Version;
use crate::gpsd::Watch;
use crate::TSReceiver;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use serde_json::Value;
use std::error::Error;
use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_util::codec::FramedRead;
use tokio_util::codec::FramedWrite;
use tracing::debug;
use tracing::error;
use tracing::info;

pub struct Client {
    server: Arc<Mutex<Server>>,
    pub addr: SocketAddr,
    req: FramedRead<OwnedReadHalf, Codec>,
    res: mpsc::Sender<Response>,
    pub watch: Arc<Mutex<Watch>>,
}

impl Client {
    pub async fn start(
        server: Arc<Mutex<Server>>,
        addr: SocketAddr,
        stream: TcpStream,
    ) -> io::Result<()> {
        let (read, write) = stream.into_split();
        let (res_tx, res_rx) = mpsc::channel(5);

        let client = Client::new(server, read, addr, res_tx).await?;

        start_client_rx(client).await;

        start_client_tx(write, res_rx).await;

        Ok(())
    }

    pub async fn new(
        server: Arc<Mutex<Server>>,
        read: OwnedReadHalf,
        addr: SocketAddr,
        res: mpsc::Sender<Response>,
    ) -> io::Result<Client> {
        let req = FramedRead::new(read, Codec::new());

        {
            let mut s = server.lock().await;

            s.clients.insert(addr, ());
        }

        let watch = Arc::new(Mutex::new(Watch::default()));

        Ok(Client {
            server,
            addr,
            req,
            res,
            watch,
        })
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        while let Some(result) = self.req.next().await {
            let command = match result {
                Ok(c) => c,
                Err(_) => Command::Error("unrecognized command".to_string()),
            };

            let response = match command {
                Command::Devices => self.command_devices().await,
                Command::Device(_) => Response::Device(Device {
                    stopbits: Some("1".to_string()),
                    ..Device::default()
                }),
                Command::Error(e) => Response::Error(ErrorMessage { message: e }),
                Command::Poll => Response::Poll(Poll {
                    time: 0.0,
                    active: 0,
                    tpv: vec![],
                    sky: vec![],
                }),
                Command::Version => Response::Version(Version {
                    release: "release-3.10".to_string(),
                    rev: "3.10".to_string(),
                    proto_major: 3,
                    proto_minor: 10,
                }),
                Command::Watch(w) => self.command_watch(w).await,
            };

            self.res.send(response).await?;
        }

        {
            let mut server = self.server.lock().await;
            server.clients.remove(&self.addr);
        }

        Ok(())
    }

    async fn command_devices(&self) -> Response {
        let devices: Devices = self.server.lock().await.devices.clone().into();

        Response::Devices(devices)
    }

    async fn command_watch(&self, updates: Option<Value>) -> Response {
        let original;
        let updated;

        {
            let mut watch = self.watch.lock().await;

            original = watch.clone();

            if let Some(j) = updates {
                watch.update(j);
            };

            updated = watch.clone();
        }

        match (original.enable, updated.enable) {
            // enable
            (false, true) => self.enable_watch(updated.clone()).await,
            // disable
            (true, false) => self.disable_watch(),
            // no change
            (true, true) => (),
            (false, false) => (),
        }

        Response::Watch(updated)
    }

    async fn enable_watch(&self, watch: Watch) {
        let mut gps_rx = None;
        let mut pps_rx = None;
        let device = match watch.device {
            Some(d) => d,
            None => return,
        };

        {
            let server = self.server.lock().await;

            if watch.enable {
                gps_rx = server.gps_rx_for(device.clone());
            }

            if watch.pps {
                pps_rx = server.pps_rx_for(device.clone())
            }
        }

        if let Some(rx) = gps_rx {
            relay_messages(self.res.clone(), rx)
        }

        if let Some(rx) = pps_rx {
            relay_pps_messages(self.res.clone(), rx)
        }
    }

    fn disable_watch(&self) {
        debug!("disabling watch for {:?}", self.addr);
    }
}

// It would be cool to use a trait here, but we can't use async with traits yet.
// https://smallcultfollowing.com/babysteps/blog/2019/10/26/async-fn-in-traits-are-hard/

fn relay_messages(tx: mpsc::Sender<Response>, rx: broadcast::Receiver<Response>) {
    tokio::spawn(async move {
        relay(tx, rx).await;
    });
}

#[tracing::instrument]
async fn relay(tx: mpsc::Sender<Response>, mut rx: broadcast::Receiver<Response>) {
    loop {
        let message = rx.recv().await;

        let value = match message {
            Ok(v) => v,
            Err(e) => {
                error!("error receiving message to relay: {:?}", e);
                break;
            }
        };

        match tx.send(value).await {
            Ok(_) => (),
            Err(e) => {
                error!("error relaying message: {:?}", e);
                break;
            }
        }
    }
}

fn relay_pps_messages(tx: mpsc::Sender<Response>, rx: TSReceiver) {
    tokio::spawn(async move {
        relay_pps(tx, rx).await;
    });
}

async fn relay_pps(tx: mpsc::Sender<Response>, mut rx: TSReceiver) {
    loop {
        let message = rx.recv().await;

        let value = match message {
            Ok(v) => v,
            Err(e) => {
                error!("error receiving message to relay: {:?}", e);
                break;
            }
        };

        match tx.send(value.into()).await {
            Ok(_) => (),
            Err(e) => {
                error!("error relaying message: {:?}", e);
                break;
            }
        }
    }
}

async fn start_client_rx(client: Client) {
    tokio::spawn(async move {
        client_rx(client).await;
    });
}

async fn client_rx(mut client: Client) {
    match client.run().await {
        Ok(_) => info!("Client {} disconnected", client.addr),
        Err(e) => error!("Error handling client {}: {:?}", client.addr, e),
    };
}

async fn start_client_tx(write: OwnedWriteHalf, rx: mpsc::Receiver<Response>) {
    let res = FramedWrite::new(write, Codec::new());

    tokio::spawn(async move {
        client_tx(res, rx).await;
    });
}

async fn client_tx(mut tx: FramedWrite<OwnedWriteHalf, Codec>, mut rx: mpsc::Receiver<Response>) {
    while let Some(value) = rx.recv().await {
        match tx.send(value).await {
            Ok(_) => (),
            Err(e) => {
                error!("Error responding to client: {:?}", e);
                break;
            }
        }
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client").field("peer", &self.addr).finish()
    }
}
