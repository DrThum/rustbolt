use std::time::Duration;

use shipyard::Unique;

#[derive(Unique, Default)]
pub struct DeltaTime(pub Duration);

impl std::ops::Deref for DeltaTime {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for DeltaTime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
