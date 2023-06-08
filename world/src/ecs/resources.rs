use std::time::Duration;

use shipyard::Unique;

#[derive(Unique, Default)]
pub struct DeltaTime(pub Duration);
