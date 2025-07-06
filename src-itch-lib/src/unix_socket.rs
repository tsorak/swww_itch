use std::{
    env::{self, VarError},
    fs,
    os::unix::net::{UnixListener, UnixStream},
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

pub fn listen() -> anyhow::Result<UnixSocket> {
    let socket_path = socket_path()?;
    // ensure::runtime_dir(&socket_path)?;

    let _ = fs::remove_file(&socket_path);

    Ok(UnixSocket::new(
        Some(UnixListener::bind(socket_path)?),
        None,
    ))
}

pub fn connect() -> anyhow::Result<UnixSocket> {
    Ok(UnixSocket::new(
        None,
        Some(UnixStream::connect(socket_path()?)?),
    ))
}
