use env_logger::Env;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // Setup logging
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    // Bind the listener to the address
    let listener = TcpListener::bind("127.0.0.1:8085").await.unwrap();

    loop {
        // The second item contains the IP and port of the new connection
        let (socket, _) = listener.accept().await.unwrap();

        // Spawn a new task for each inbound socket
        tokio::spawn(async move {
            rustbolt_world::process(socket).await.expect("World error");
        });
    }
}
