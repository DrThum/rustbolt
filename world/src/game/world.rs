use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::time::interval;

use crate::{config::WorldConfig, datastore::DataStore, world_session::WorldSession};

pub struct World {
    pub data_store: DataStore,
    start_time: Instant,
    config: Arc<WorldConfig>,
    sessions: HashMap<u32, WorldSession>,
}

impl World {
    pub fn new(config: Arc<WorldConfig>, pool: Arc<Pool<SqliteConnectionManager>>) -> Self {
        let conn = pool.get().unwrap();
        let data_store = DataStore::load_data(&config.common.data, &conn)
            .expect("Error when loading static data");

        World {
            data_store,
            start_time: Instant::now(),
            config,
            sessions: HashMap::new(),
        }
    }

    pub async fn start(&'static self) {
        tokio::spawn(async move {
            self.game_loop().await;
        });
    }

    pub async fn insert_session(&mut self, session: WorldSession) -> Option<WorldSession> {
        self.sessions.insert(session.account_id, session)
    }

    pub async fn get_session_for_account(&mut self, account_id: u32) -> Option<&mut WorldSession> {
        self.sessions.get_mut(&account_id)
    }

    // Return the elapsed time since the World started
    pub fn game_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn config(&self) -> &WorldConfig {
        &self.config
    }

    async fn game_loop(&self) {
        let mut interval = interval(Duration::from_millis(50));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut time = Instant::now();

        loop {
            let new_time = Instant::now();
            let diff = new_time.duration_since(time);

            time = new_time;
            self.tick(diff);

            interval.tick().await;
        }
    }

    fn tick(&self, _diff: Duration) {}
}
