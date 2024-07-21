use shipyard::Component;

// The Unwind component is used to mark a creature when the last player that was around it is
// gone. The creature can then unwind (or reset) and only update a few of its systems.
#[derive(Component, Default)]
pub struct Unwind {}
