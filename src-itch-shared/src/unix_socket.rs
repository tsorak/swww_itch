use serde::{Deserialize, Serialize};
use std::path::Path;

mod internal;
mod setup;
pub use self::{
    internal::{Listener, UnixSocket},
    setup::{IntoUnixSocketPath, UnixSocketPath},
};

/// Resolves once the listener is successfully bound.
pub async fn setup_listener<REQ, P: AsRef<Path>>(listen_path: P) -> UnixSocket<REQ, ()>
where
    REQ: Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    let mut socket = UnixSocket::new(None, None);
    while !socket.can_recv() {
        match socket.listen(listen_path.as_ref()) {
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
    P: AsRef<Path>,
>(
    p: P,
) -> UnixSocket<REQ, RES> {
    let mut socket = UnixSocket::new(None, None);
    let _ = socket.connect(p.as_ref());
    socket
}
