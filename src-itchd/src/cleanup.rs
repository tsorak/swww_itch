use std::path::PathBuf;
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
};

pub struct Cleanup {
    pub unix_socket_path: PathBuf,
}

impl Cleanup {
    pub fn bind_os_signals(self) {
        tokio::spawn(async move {
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            let mut sigint = signal(SignalKind::interrupt()).unwrap();
            select! {
                _ = sigterm.recv() => println!("Received SIGTERM"),
                _ = sigint.recv() => println!("Received SIGINT"),
            }

            match std::fs::remove_file(self.unix_socket_path) {
                Ok(_) => {
                    println!("Removed socket")
                }
                Err(_) => {
                    println!("Failed to remove socket")
                }
            }

            std::process::exit(0);
        });
    }
}
