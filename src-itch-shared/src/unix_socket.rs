use std::{
    env::{self, VarError},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

mod internal;
pub use internal::{Listener, UnixSocket};

pub fn socket_path() -> Result<PathBuf, VarError> {
    Ok(PathBuf::from(env::var("XDG_RUNTIME_DIR")?).join("swwwitch.sock"))
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    SwitchToBackground(String),
    EditQueue,
}

/// Resolves once the listener is successfully bound.
pub async fn setup_listener() -> UnixSocket {
    let mut socket = UnixSocket::default();
    while !socket.can_recv() {
        match socket.listen() {
            Ok(_) => {
                break;
            }
            Err(err) => {
                eprintln!("Listener failed to bind: {err}");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                println!("Retrying...")
            }
        }
    }
    socket
}

pub fn connect() -> UnixSocket {
    let mut socket = UnixSocket::default();
    let _ = socket.connect();
    socket
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SwitchToBackground(p) => {
                write!(f, "SwitchToBackground '{p}'")
            }
            Self::EditQueue => {
                write!(f, "EditQueue")
            }
        }
    }
}
