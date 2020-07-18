use crate::JsonSender;

use json::parse;
use json::stringify;

use tokio::io::BufReader;
use tokio::io::BufWriter;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::prelude::*;

use tracing::error;
use tracing::debug;
use tracing::info;

pub struct WatchData {
    pub device: String,
    pub enable: bool,
    pub pps:    bool,
}

enum Request {
    Version,
    Watch(WatchData),
}

#[tracing::instrument]
pub fn spawn(port: u16, tx: JsonSender) {
    tokio::spawn(async move {
        let address = ("0.0.0.0", port);

        let mut listener = TcpListener::bind(address).await.unwrap();

        loop {
            info!("waiting for client");

            let (socket, _) = listener.accept().await.unwrap();

            info!("client connected {:?}", socket.peer_addr().unwrap());

            handle_client(socket, tx.clone());
        }
    });
}

#[tracing::instrument]
fn handle_client(mut socket: TcpStream, tx: JsonSender) {
    tokio::spawn(async move {
        match socket.set_nodelay(true) {
            Ok(_) => (),
            Err(e) => {
                error!("enabling NODELAY {:?}", e);
                return;
            },
        };

        let (recv, send) = socket.split();

        let recv = BufReader::new(recv);
        let mut lines = recv.lines();

        let mut send = BufWriter::new(send);

        loop {
            let request = match lines.next_line().await {
                Ok(l) => l,
                Err(e) => {
                    error!("unable to get next line {:?}", e);
                    break;
                },
            };

            debug!("request: {:?}", request);

            let request = match handle_request(request) {
                Ok(r) => r,
                Err(e) => {
                    error!("{}", e);
                    continue;
                },
            };

            let response = match request {
                Request::Version =>
                    Some(
                        format!("{}\n",
                            stringify(
                                json::object!{
                                    class: "VERSION",
                                    release: "where_am_i 0.0.0",
                                    rev: "",
                                    proto_major: 3,
                                    proto_minor: 10,
                                }))),
                Request::Watch(w) => {
                    info!("watching enabled: {:?} pps: {:?}", w.enable, w.pps);

                    if w.enable && w.pps {
                        let mut rx = tx.subscribe();

                        debug!("subscribed to messages {:?}", rx);

                        loop {
                            debug!("waiting for message");
                            let json = match rx.recv().await {
                                Ok(j) => j,
                                Err(e) => {
                                    error!("receiving message from channel {:?}", e);
                                    break;
                                }
                            };

                            let message = format!("{}\n", stringify(json));

                            info!("sending to client: {}", message);

                            match send.write(message.as_bytes()).await {
                                Ok(_) => (),
                                Err(e) => {
                                    error!("sending message to client {:?}", e);
                                    break;
                                },
                            };

                            match send.flush().await {
                                Ok(_) => (),
                                Err(e) => {
                                    error!("flushing socket {:?}", e);
                                    break;
                                },
                            }
                        }

                        debug!("done relaying messages");
                    }

                    None
                },
            };

            match response {
                Some(r) =>
                    match send.write(r.as_bytes()).await {
                        Ok(_) => (),
                        Err(e) => {
                            error!("sending version {:?}", e);
                            break;
                        },
                    },
                None => (),
            };


            match send.flush().await {
                Ok(_) => (),
                Err(e) => {
                    error!("flushing socket {:?}", e);
                    break;
                },
            };
        }

        debug!("Hanging up on client");
    });
}

#[tracing::instrument]
fn handle_request(request: Option<String>) -> Result<Request, String> {
    let request = match request {
        Some(r) => r,
        None    => return Err("line not received".to_string()),
    };

    if request == "?VERSION;" {
        Ok(Request::Version)
    } else if request.starts_with("?WATCH=") {
        let json_start = match request.find("{") {
            Some(i) => i,
            None    => return Err("missing {{ in WATCH request".to_string()),
        };

        let json_end = match request.rfind("}") {
            Some(i) => i,
            None    => return Err("missing }} in WATCH request".to_string()),
        };

        let watch_json = match request.get(json_start..=json_end) {
            Some(j) => j,
            None    => return Err("error extracting WATCH JSON".to_string()),
        };

        let watch = match parse(watch_json) {
            Ok(w)  => w,
            Err(_) => return Err(format!("Error parsing WATCH body {}", watch_json)),
        };

        let device = watch["device"].as_str()
            .unwrap_or_else(|| "UNSET").to_string();

        let enable = watch["enable"].as_bool()
            .unwrap_or_else(|| false);

        let pps = watch["pps"].as_bool()
            .unwrap_or_else(|| false);

        Ok(Request::Watch(WatchData { device, enable, pps }))
    } else {
        Err(format!("unknown request: {}", request))
    }
}
