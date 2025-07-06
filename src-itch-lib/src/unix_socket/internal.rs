use std::{
    io::Read,
    os::unix::net::{UnixListener, UnixStream},
};

use serde::{Deserialize, Serialize};
use tokio::{sync::mpsc, task::spawn_blocking};

pub struct UnixSocket {
    pub listener: Option<Listener>,
    pub tx: Option<UnixStream>,
}

impl UnixSocket {
    pub(super) fn new(listener: Option<UnixListener>, tx: Option<UnixStream>) -> Self {
        let mut maybe_listener = None;

        if let Some(l) = listener {
            let _ = maybe_listener.insert(Listener::new(l));
        }

        Self {
            listener: maybe_listener,
            tx,
        }
    }

    pub async fn recv<T: for<'a> Deserialize<'a>>(&mut self) -> Option<T> {
        match self.listener.as_mut() {
            Some(l) => match l.rx.recv().await {
                Some(s) => serde_json::from_str(&s)
                    .inspect_err(|_| eprintln!("Received unexpected data over socket. Skipping"))
                    .ok(),
                None => None,
            },
            None => None,
        }
    }
}

/// The Listener will receive messages from all connected clients and join them into a single channel.
pub struct Listener {
    //listener: UnixListener,
    rx: mpsc::UnboundedReceiver<String>,
}

impl Listener {
    fn new(l: UnixListener) -> Self {
        let (tx, rx) = mpsc::unbounded_channel::<String>();

        spawn_blocking(move || {
            for stream in l.incoming().flatten() {
                let tx = tx.clone();
                spawn_blocking(move || join_messages(stream, tx));
            }
        });

        Self { rx }
    }
}

fn join_messages(mut of: UnixStream, to: mpsc::UnboundedSender<String>) {
    loop {
        let mut buf = String::new();
        match of.read_to_string(&mut buf) {
            Ok(n) => {
                if n == 0 {
                    eprintln!("Received empty message over socket");
                    continue;
                }
            }
            Err(_) => continue,
        }

        let _ = to.send(buf);
    }
}
