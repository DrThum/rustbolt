use std::sync::Arc;

use env_logger::Env;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // Setup logging
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    // Setup database connection pool
    let sqlite_connection_manager = SqliteConnectionManager::file("./data/databases/auth.db");
    let db_pool = r2d2::Pool::new(sqlite_connection_manager)
        .expect("Failed to create r2d2 SQlite connection pool");
    let db_pool = Arc::new(db_pool);

    // Bind the listener to the address
    let listener = TcpListener::bind("127.0.0.1:8085").await.unwrap();

    loop {
        // The second item contains the IP and port of the new connection
        let (socket, _) = listener.accept().await.unwrap();

        // Spawn a new task for each inbound socket
        let db_pool_copy = Arc::clone(&db_pool);
        tokio::spawn(async move {
            rustbolt_world::process(socket, db_pool_copy)
                .await
                .expect("World error");
        });
    }
}
