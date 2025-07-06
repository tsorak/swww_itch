use std::{
    fs,
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
};

use serde::{Deserialize, Serialize};
use tokio::{sync::mpsc, task::spawn_blocking};

use super::socket_path;

#[derive(Default)]
pub struct UnixSocket {
    pub listener: Option<Listener>,
    pub tx: Option<UnixStream>,
}

impl UnixSocket {
    #[allow(unused)]
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

    pub fn listen(&mut self) -> anyhow::Result<()> {
        let socket_path = socket_path()?;
        // ensure::runtime_dir(&socket_path)?;

        let _ = fs::remove_file(&socket_path);

        let listener = UnixListener::bind(socket_path)?;
        let _ = self.listener.insert(Listener::new(listener));

        Ok(())
    }

    pub fn connect(&mut self) -> anyhow::Result<()> {
        let socket_path = socket_path()?;
        let stream = UnixStream::connect(socket_path)?;
        let _ = self.tx.insert(stream);

        Ok(())
    }

    pub fn can_send(&self) -> bool {
        self.tx.is_some()
    }

    pub fn can_recv(&self) -> bool {
        self.listener.is_some()
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

    pub async fn send<T: Serialize>(&mut self, msg: T) -> Result<(), std::io::Error> {
        match self.tx.as_mut() {
            Some(tx) => {
                //self note: Just because tx is Some it does not mean the actual socket file exists.
                // the server may have disconnected.
                // Show this somewhere in tauri app + retry connection button

                let msg = serde_json::to_string(&msg)?;
                tx.write_all(msg.as_bytes())?;
                tx.flush()?;
                Ok(())
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "No connection established",
            )),
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
        let _ = to.send(string_data);
    }
}
