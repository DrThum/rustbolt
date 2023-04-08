use std::{sync::Arc, time::Instant};

use env_logger::Env;
use r2d2_sqlite::SqliteConnectionManager;
use rustbolt_world::{
    config::WorldConfig, database_context::DatabaseContext, game::world::World,
    world_context::WorldContext, world_session::OpcodeHandler, DataStore, SessionHolder,
};
use tokio::{
    net::TcpListener,
    sync::{Mutex, RwLock, Semaphore},
};

mod embedded_characters {
    use refinery::embed_migrations;
    embed_migrations!("../sql_migrations/characters");
}

mod embedded_world {
    use refinery::embed_migrations;
    embed_migrations!("../sql_migrations/world");
}

#[tokio::main]
async fn main() {
    // Load config
    let config = Arc::new(WorldConfig::load().expect("Error in config file"));

    // Setup logging
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    // Setup database connection pools
    let db_pool_auth = r2d2::Pool::new(SqliteConnectionManager::file(format!(
        "{}/databases/auth.db",
        config.common.data.directory
    )))
    .expect("Failed to create r2d2 SQlite connection pool (Auth DB)");
    let db_pool_auth = Arc::new(db_pool_auth);

    let mutex = Semaphore::new(1);
    let sqlite_connection_manager_char = SqliteConnectionManager::file(format!(
        "{}/databases/characters.db",
        config.common.data.directory
    ))
    .with_init(move |c| {
        if let Ok(_) = mutex.try_acquire() {
            embedded_characters::migrations::runner().run(c).unwrap();
        }
        Ok(())
    });
    let db_pool_char = r2d2::Pool::new(sqlite_connection_manager_char)
        .expect("Failed to create r2d2 SQlite connection pool (Characters DB)");
    let db_pool_char = Arc::new(db_pool_char);

    let mutex = Semaphore::new(1);
    let sqlite_connection_manager_world = SqliteConnectionManager::file(format!(
        "{}/databases/world.db",
        config.common.data.directory
    ))
    .with_init(move |c| {
        if let Ok(_) = mutex.try_acquire() {
            embedded_world::migrations::runner().run(c).unwrap();
        }
        Ok(())
    });
    let db_pool_world = r2d2::Pool::new(sqlite_connection_manager_world)
        .expect("Failed to create r2d2 SQlite connection pool (World DB)");
    let db_pool_world = Arc::new(db_pool_world);

    let conn = db_pool_world.get().unwrap();
    let data_store = Arc::new(
        DataStore::load_data(&config.common.data, &conn).expect("Error when loading static data"),
    );
    let opcode_handler = Arc::new(OpcodeHandler::new());

    let database_context = Arc::new(DatabaseContext {
        auth: db_pool_auth.clone(),
        characters: db_pool_char.clone(),
        world: db_pool_world.clone(),
    });

    let start_time = Instant::now();

    let world_context = Arc::new(WorldContext {
        data_store,
        database: database_context,
        opcode_handler: opcode_handler.clone(),
        config: config.clone(),
        start_time,
    });

    let session_holder = Arc::new(RwLock::new(SessionHolder::new()));

    let world = World::new(start_time, config.clone());

    let world: Arc<RwLock<World>> = Arc::new(RwLock::new(world));
    World::start(world.clone()).await;

    // Bind the listener to the address
    let listener = TcpListener::bind(format!(
        "{}:{}",
        config.world.network.host, config.world.network.port
    ))
    .await
    .unwrap();

    tokio::spawn(async move {
        loop {
            // The second item contains the IP and port of the new connection
            let (socket, _) = listener.accept().await.unwrap();

            let world_context = world_context.clone();
            let session_holder = session_holder.clone();

            // Spawn a new task for each inbound socket
            tokio::spawn(async move {
                rustbolt_world::process(socket, world_context, session_holder)
                    .await
                    .expect("World socket error");
            });
        }
    });

    loop {} // TODO: join
}
