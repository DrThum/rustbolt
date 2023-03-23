use std::sync::Arc;

use env_logger::Env;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::{net::TcpListener, sync::Mutex};

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("../sql_migrations/characters");
}

#[tokio::main]
async fn main() {
    // Setup logging
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    // Setup database connection pools
    let db_pool_auth = r2d2::Pool::new(SqliteConnectionManager::file("./data/databases/auth.db"))
        .expect("Failed to create r2d2 SQlite connection pool (Auth DB)");
    let db_pool_auth = Arc::new(db_pool_auth);

    let sqlite_connection_manager_char =
        SqliteConnectionManager::file("./data/databases/characters.db").with_init(|c| {
            embedded::migrations::runner().run(c).unwrap();
            Ok(())
        });
    let db_pool_char = r2d2::Pool::new(sqlite_connection_manager_char)
        .expect("Failed to create r2d2 SQlite connection pool (Characters DB)");
    let db_pool_char = Arc::new(db_pool_char);

    // Bind the listener to the address
    let listener = TcpListener::bind("127.0.0.1:8085").await.unwrap();

    loop {
        // The second item contains the IP and port of the new connection
        let (socket, _) = listener.accept().await.unwrap();

        // Spawn a new task for each inbound socket
        let db_pool_auth_copy = Arc::clone(&db_pool_auth);
        let db_pool_char_copy = Arc::clone(&db_pool_char);

        // let session = Arc::new(WorldSession::new(
        //     Arc::new(Mutex::new(socket)),
        //     db_pool_auth_copy,
        //     db_pool_char_copy,
        // ));
        tokio::spawn(async move {
            rustbolt_world::process(
                Arc::new(Mutex::new(socket)),
                db_pool_auth_copy,
                db_pool_char_copy,
            )
            .await
            .expect("World error");
        });
    }
}
