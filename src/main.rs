use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // Bind the listener to the address
    let listener = TcpListener::bind("127.0.0.1:3724").await.unwrap();

    loop {
        // The second item contains the IP and port of the new connection
        let (socket, _) = listener.accept().await.unwrap();
        // Spawn a new task for each inbound socket
        tokio::spawn(async move {
            rustbolt_auth::process(socket).await;
        });
    }
}
