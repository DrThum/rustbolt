use env_logger::Env;
use log::trace;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use rustbolt_auth::{config::AuthConfig, AuthError, Realm, RealmType};
use std::sync::Arc;
use tokio::net::TcpListener;

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("../sql_migrations/auth");
}

#[tokio::main]
async fn main() {
    // Load config
    let config = AuthConfig::load().expect("Error in config file");

    // Setup logging
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    // Setup database connection pool and execute migrations
    let sqlite_connection_manager = SqliteConnectionManager::file(format!(
        "{}/databases/auth.db",
        config.common.data.directory
    ))
    .with_init(|c| {
        embedded::migrations::runner().run(c).unwrap();
        Ok(())
    });
    let db_pool = r2d2::Pool::new(sqlite_connection_manager)
        .expect("Failed to create r2d2 SQlite connection pool");
    let db_pool = Arc::new(db_pool);

    // Load realms from database
    let mut conn = db_pool.get().unwrap();
    let realms = load_realms_from_db(&mut conn);

    // Bind the listener to the address
    let listener = TcpListener::bind(format!(
        "{}:{}",
        config.auth.network.host, config.auth.network.port
    ))
    .await
    .unwrap();

    loop {
        // The second item contains the IP and port of the new connection
        let (socket, _) = listener.accept().await.unwrap();

        // Spawn a new task for each inbound socket
        let realms_copy = Arc::clone(&realms);
        let db_pool_copy = Arc::clone(&db_pool);
        tokio::spawn(async move {
            match rustbolt_auth::process(socket, realms_copy, db_pool_copy).await {
                Ok(_) => (),
                Err(AuthError::ClientDisconnected) => trace!("Client disconnected"),
                Err(e) => panic!("Parse error during auth sequence: {:?}", e),
            }
        });
    }
}

fn load_realms_from_db(conn: &mut Connection) -> Arc<Vec<Realm>> {
    let mut stmt = conn.prepare("SELECT id, realm_type, is_locked, flags, name, address, population, category FROM realms").unwrap();
    let realm_iter = stmt
        .query_map([], |row| {
            let realm_type: i32 = row.get("realm_type")?;
            let realm_type = RealmType::try_from(realm_type).unwrap();
            let locked: i32 = row.get("is_locked")?;
            let locked = locked > 0;
            let realm_name: String = row.get("name")?;
            let address: String = row.get("address")?;

            Ok(Realm {
                _realm_type: realm_type,
                _locked: locked,
                _realm_flags: row.get("flags")?,
                _realm_name: realm_name.try_into().unwrap(),
                _address_port: address.try_into().unwrap(),
                _population: row.get("population")?,
                _num_chars: 1,
                _realm_category: row.get("category")?,
                _realm_id: row.get("id")?,
            })
        })
        .unwrap();

    Arc::new(realm_iter.map(|r| r.unwrap()).collect())
}
