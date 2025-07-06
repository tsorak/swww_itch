use swww_itch_lib::unix_socket::{Message, listen};

mod cleanup;

#[tokio::main]
async fn main() {
    cleanup::bind_os_signals();

    let mut listener = listen().unwrap();

    println!("Waiting for messages");
    while let Some(_msg) = listener.recv::<Message>().await {
        println!("Received message");
    }
}
