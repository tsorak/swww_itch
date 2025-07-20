use std::{fs, path::Path, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    sync::{Mutex, broadcast, mpsc},
};

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

    pub fn listen<P: AsRef<Path>>(&mut self, p: P) -> anyhow::Result<()> {
        let _ = fs::remove_file(p.as_ref());

        let listener = UnixListener::bind(p.as_ref())?;
        let _ = self.listener.insert(Listener::new(listener));

        Ok(())
    }

    pub async fn connect<P: AsRef<Path>>(&mut self, p: P) -> anyhow::Result<()>
    where
        REQ: Serialize + Send + 'static,
        RES: for<'de> Deserialize<'de> + Clone + Send + 'static,
    {
        let stream = UnixStream::connect(p.as_ref()).await?;
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

        let (r_stream, w_stream) = stream.into_split();

        let _request_handler = tokio::spawn(async move {
            let mut req_rx = req_rx;
            let mut w_stream = w_stream;

            loop {
                if let Some(v) = req_rx.recv().await {
                    let _ = w_stream
                        .write_all(&serde_json::to_vec(&Message::<REQ, ()>::Request(v)).unwrap())
                        .await;
                }
            }
        });

        let res_tx2 = res_tx.clone();

        let _response_handler = tokio::spawn(async move {
            let res_tx = res_tx2;
            let mut r_stream = r_stream;

            loop {
                let mut buf = vec![0; 1024];
                if let Ok(n_bytes) = r_stream.read(&mut buf).await {
                    let s = String::from_utf8_lossy(&buf[..n_bytes]).trim().to_string();

                    if let Ok(Message::Response(res)) = serde_json::from_str::<Message<(), RES>>(&s)
                    {
                        match res_tx.send(res) {
                            Ok(_) => {}
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
    rx: mpsc::UnboundedReceiver<(Arc<Mutex<UnixStream>>, String)>,
}

impl Listener {
    fn new(l: UnixListener) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            loop {
                if let Ok(stream) = l.accept().await {
                    let tx = tx.clone();
                    tokio::spawn(async move { join_messages(stream.0, tx).await });
                }
            }
        });

        Self { rx }
    }
}

async fn join_messages(
    of: UnixStream,
    to: mpsc::UnboundedSender<(Arc<Mutex<UnixStream>>, String)>,
) {
    let of = Arc::new(Mutex::new(of));
    loop {
        // Compiler seems to think this is unused
        #[allow(unused_assignments)]
        let mut string_data = String::new();

        let mut lock = of.lock().await;

        let mut buf = vec![0; 1024];
        match lock.read(&mut buf).await {
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

        drop(lock);

        // println!("Joining message '{string_data}' to Listener");
        let _ = to.send((of.clone(), string_data));
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Message<REQ, RES> {
    Request(REQ),
    Response(RES),
}

pub struct RequestContext<T: for<'de> Deserialize<'de>> {
    peer: Arc<Mutex<UnixStream>>,
    _request_string: String,
    request: Option<T>,
}

impl<T: for<'de> Deserialize<'de>> RequestContext<T> {
    pub fn take_request(&mut self) -> T {
        self.request.take().unwrap()
    }

    pub async fn respond<R: Serialize>(self, res: R) -> anyhow::Result<()> {
        let msg = serde_json::to_vec(&Message::<(), R>::Response(res))?;

        let mut lock = self.peer.lock().await;
        lock.write_all(&msg).await?;

        Ok(())
    }
}
