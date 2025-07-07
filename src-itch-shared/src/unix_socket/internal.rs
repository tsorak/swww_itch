use std::{
    fs,
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
};

use serde::{Deserialize, Serialize};
use tokio::{
    sync::{broadcast, mpsc},
    task::spawn_blocking,
};

use super::socket_path;

#[derive(Default)]
pub struct UnixSocket<REQ, RES>
where
    REQ: Serialize + Send + 'static,
    RES: for<'de> Deserialize<'de> + Clone + Send + 'static,
{
    pub listener: Option<Listener>,
    pub connection: Option<Connection<REQ, RES>>,
}

impl<
    REQ: for<'de> Deserialize<'de> + Serialize + Send + 'static,
    RES: for<'de> Deserialize<'de> + Clone + Send + 'static,
> UnixSocket<REQ, RES>
{
    #[allow(unused)]
    pub(super) fn new(listener: Option<UnixListener>, stream: Option<UnixStream>) -> Self {
        let mut maybe_listener = None;

        if let Some(l) = listener {
            let _ = maybe_listener.insert(Listener::new(l));
        }

        let mut maybe_connection = None;

        if let Some(s) = stream {
            let _ = maybe_connection.insert(Connection::new(s));
        }

        Self {
            listener: maybe_listener,
            connection: maybe_connection,
        }
    }

    pub fn listen(&mut self) -> anyhow::Result<()> {
        let socket_path = socket_path()?;
        // ensure::runtime_dir(&socket_path)?;

        let _ = fs::remove_file(&socket_path);

        let listener = UnixListener::bind(socket_path)?;
        let _ = self.listener.insert(Listener::new(listener));

        Ok(())
    }

    pub fn connect(&mut self) -> anyhow::Result<()>
    where
        REQ: Serialize + Send + 'static,
        RES: for<'de> Deserialize<'de> + Clone + Send + 'static,
    {
        let socket_path = socket_path()?;
        let stream = UnixStream::connect(socket_path)?;
        let _ = self.connection.insert(Connection::<REQ, RES>::new(stream));

        Ok(())
    }

    pub fn can_send(&self) -> bool {
        self.connection.is_some()
    }

    pub fn can_recv(&self) -> bool {
        self.listener.is_some()
    }

    pub async fn recv(&mut self) -> Option<RequestContext<REQ>> {
        // Some if we are a server
        match self.listener.as_mut() {
            // Some unless stream closed
            Some(l) => match l.rx.recv().await {
                Some((peer, s)) => {
                    let msg = serde_json::from_str::<Message<REQ, ()>>(&s)
                        .inspect_err(|_| {
                            eprintln!("Received unexpected data over socket. Skipping")
                        })
                        .ok()?;

                    if let Message::Request(req) = msg {
                        Some(RequestContext {
                            peer,
                            _request_string: s,
                            request: Some(req),
                        })
                    } else {
                        None
                    }
                }
                None => None,
            },
            None => None,
        }
    }
}

pub struct Connection<REQ, RES>
where
    REQ: Serialize + Send + 'static,
    RES: for<'de> Deserialize<'de> + Clone + Send + 'static,
{
    req_tx: mpsc::UnboundedSender<REQ>,
    res_tx: broadcast::Sender<RES>,
}

impl<REQ: Serialize + Send + 'static, RES: for<'de> Deserialize<'de> + Clone + Send + 'static>
    Connection<REQ, RES>
{
    fn new(stream: UnixStream) -> Self {
        let (req_tx, req_rx) = mpsc::unbounded_channel::<REQ>();
        let (res_tx, _res_rx) = broadcast::channel(8);

        let tx_stream = stream
            .try_clone()
            .expect("Failed to clone UnixStream for transmitting end");
        let _request_handler = tokio::spawn(async move {
            let mut req_rx = req_rx;
            let mut stream = tx_stream;

            loop {
                if let Some(v) = req_rx.recv().await {
                    let _ = stream
                        .write_all(&serde_json::to_vec(&Message::<REQ, ()>::Request(v)).unwrap());
                }
            }
        });

        let res_tx2 = res_tx.clone();

        let _response_handler = spawn_blocking(move || {
            let res_tx = res_tx2;
            let mut stream = stream;

            loop {
                let mut buf = vec![0; 1024];
                if let Ok(n_bytes) = stream.read(&mut buf) {
                    let s = String::from_utf8_lossy(&buf[..n_bytes]).trim().to_string();

                    if let Ok(Message::Response(res)) = serde_json::from_str::<Message<(), RES>>(&s)
                    {
                        match res_tx.send(res) {
                            Ok(_) => {
                                println!("Transmitted response");
                            }
                            Err(_) => {
                                eprintln!("Received response from peer which we didn't handle");
                            }
                        };
                    }
                }
            }
        });

        Self { req_tx, res_tx }
    }

    pub fn send_request(&mut self, req: REQ) -> Result<(), mpsc::error::SendError<REQ>> {
        self.req_tx.send(req)
    }

    /// Receive a single response
    pub async fn receive_response(&mut self) -> Result<RES, broadcast::error::RecvError> {
        self.res_tx.subscribe().recv().await
    }

    pub async fn take_response(&mut self, cmp: impl Fn(&RES) -> bool + Send) -> RES {
        let mut rx = self.res_tx.subscribe();

        loop {
            let res = rx.recv().await.unwrap();

            if cmp(&res) {
                return res;
            }
        }
    }
}

/// The Listener will receive messages from all connected clients and join them into a single channel.
pub struct Listener {
    //listener: UnixListener,
    rx: mpsc::UnboundedReceiver<(UnixStream, String)>,
}

impl Listener {
    fn new(l: UnixListener) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        spawn_blocking(move || {
            for stream in l.incoming().flatten() {
                let tx = tx.clone();
                spawn_blocking(move || join_messages(stream, tx));
            }
        });

        Self { rx }
    }
}

fn join_messages(mut of: UnixStream, to: mpsc::UnboundedSender<(UnixStream, String)>) {
    loop {
        // Compiler seems to think this is unused
        #[allow(unused_assignments)]
        let mut string_data = String::new();

        let mut buf = vec![0; 1024];
        match of.read(&mut buf) {
            Ok(n_bytes) => {
                let data = String::from_utf8_lossy(&buf[..n_bytes]).trim().to_string();

                if data.is_empty() {
                    eprintln!("Received empty message. Closing connection...");
                    break;
                }

                string_data = data;
            }
            Err(_) => {
                eprintln!("Error reading socket input. Closing connection...");
                break;
            }
        }

        // println!("Joining message '{string_data}' to Listener");
        let _ = to.send((
            of.try_clone()
                .expect("Failed to clone socket handle for replying"),
            string_data,
        ));
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Message<REQ, RES> {
    Request(REQ),
    Response(RES),
}

pub struct RequestContext<T: for<'de> Deserialize<'de>> {
    peer: UnixStream,
    _request_string: String,
    request: Option<T>,
}

impl<T: for<'de> Deserialize<'de>> RequestContext<T> {
    pub fn take_request(&mut self) -> T {
        self.request.take().unwrap()
    }

    pub fn respond<R: Serialize>(mut self, res: R) -> anyhow::Result<()> {
        Ok(self
            .peer
            .write_all(&serde_json::to_vec(&Message::<(), R>::Response(res))?)?)
    }
}
