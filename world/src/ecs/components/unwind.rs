use shipyard::Component;

// The Unwind component is used to mark a creature when the last player that was around it is
// gone. The creature can then unwind (or reset) and only update a few of its systems.
#[derive(Component)]
pub struct Unwind {}

// TODO:
// - keep unwind for a short time (30 seconds?) then remove it
// - remove it upon player aggro (when adding NearbyPlayers)
// - systems:
//   - movement: trigger MoveToHome when Unwind is active
//   - combat: stop all combat (and notify creature/player enemies)
