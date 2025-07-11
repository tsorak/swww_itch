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
#[serde(rename_all = "camelCase")]
pub enum Request {
    SwitchToBackground(String),
    RearrangeBackground((String, String, String)),
    GetQueue,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Response {
    SwitchToBackground(bool),
    RearrangeBackground((bool, usize, usize)),
    GetQueue(Vec<String>),
}

/// Resolves once the listener is successfully bound.
pub async fn setup_listener<REQ>() -> UnixSocket<REQ, ()>
where
    REQ: Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    let mut socket = UnixSocket::new(None, None);
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

pub fn connect<
    REQ: Serialize + for<'de> Deserialize<'de> + Send + 'static,
    RES: for<'de> Deserialize<'de> + Clone + Send + 'static,
>() -> UnixSocket<REQ, RES> {
    let mut socket = UnixSocket::new(None, None);
    let _ = socket.connect();
    socket
}
