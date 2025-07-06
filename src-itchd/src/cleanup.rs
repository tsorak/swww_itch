use swww_itch_lib::unix_socket::socket_path;
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
};

pub fn bind_os_signals() {
    tokio::spawn(async {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        select! {
            _ = sigterm.recv() => println!("Received SIGTERM"),
            _ = sigint.recv() => println!("Received SIGINT"),
        }

        if let Ok(p) = socket_path() {
            match std::fs::remove_file(p) {
                Ok(_) => {
                    println!("Removed socket")
                }
                Err(_) => {
                    println!("Failed to remove socket")
                }
            }
        }

        std::process::exit(0);
    });
}
