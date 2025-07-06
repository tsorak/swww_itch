use swww_itch_shared::{
    swww_ffi,
    unix_socket::{Message, setup_listener},
};

mod cleanup;

#[tokio::main]
async fn main() {
    cleanup::bind_os_signals();

    let mut listener = setup_listener().await;

    println!("Waiting for connections...");
    loop {
        if let Some(msg) = listener.recv::<Message>().await {
            println!("Received job: {msg}");
            match msg {
                Message::SwitchToBackground(p) => {
                    swww_ffi::set_background(&p);
                }
                _ => {}
            }
        }
    }
}
