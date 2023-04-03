use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::time::interval;

use crate::{config::WorldConfig, datastore::DataStore};

pub struct World {
    pub data_store: DataStore,
}

impl World {
    pub fn new(config: &WorldConfig, pool: Arc<Pool<SqliteConnectionManager>>) -> Self {
        let conn = pool.get().unwrap();
        let data_store = DataStore::load_data(&config.common.data, &conn)
            .expect("Error when loading static data");

        World { data_store }
    }

    pub async fn start(&'static self) {
        tokio::spawn(async move {
            self.game_loop().await;
        });
    }

    async fn game_loop(&self) {
        let mut interval = interval(Duration::from_millis(50));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut time = SystemTime::now();

        loop {
            let new_time = SystemTime::now();
            if let Ok(diff) = new_time.duration_since(time) {
                time = new_time;
                self.tick(diff);
            }

            interval.tick().await;
        }
    }

    fn tick(&self, _diff: Duration) {}
}
