use core::fmt;
use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;

use crate::{create_wrapped_resource, entities::object_guid::ObjectGuid};

use super::world_session::WorldSession;

pub struct SessionHolder<Key: Eq + std::hash::Hash + fmt::Debug> {
    sessions: RwLock<HashMap<Key, Arc<WorldSession>>>,
}

impl<Key> Default for SessionHolder<Key>
where
    Key: Eq + std::hash::Hash + fmt::Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Key> SessionHolder<Key>
where
    Key: Eq + std::hash::Hash + fmt::Debug,
{
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert_session(
        &self,
        key: Key,
        session: Arc<WorldSession>,
    ) -> Option<Arc<WorldSession>> {
        self.sessions.write().insert(key, session)
    }

    pub fn remove_session(&self, key: &Key) {
        if self.sessions.write().remove(key).is_none() {
            log::warn!(
                "trying to remove a non-existent session: account id or player guid = {:?}",
                key
            );
        }
    }

    pub fn get_session(&self, key: &Key) -> Option<Arc<WorldSession>> {
        self.sessions.read().get(key).cloned()
    }

    pub fn get_matching_sessions(
        &self,
        predicate: impl Fn(&Key) -> bool,
    ) -> Vec<Arc<WorldSession>> {
        self.sessions
            .read()
            .iter()
            .filter(|(key, _session)| predicate(key))
            .map(|(_, session)| session.clone())
            .collect()
    }
}

create_wrapped_resource!(WrappedSessionHolder, SessionHolder<ObjectGuid>);
