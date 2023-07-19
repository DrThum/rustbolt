use shipyard::{IntoIter, UniqueView, View, ViewMut};

use crate::{
    ecs::{
        components::{guid::Guid, movement::Movement},
        resources::DeltaTime,
    },
    entities::{
        creature::Creature,
        object_guid::ObjectGuid,
        player::Player,
        position::{Position, WorldPosition},
    },
    game::{map::WrappedMap, movement_spline::MovementSplineState},
};

pub fn update_movement(
    dt: UniqueView<DeltaTime>,
    map: UniqueView<WrappedMap>,
    v_guid: View<Guid>,
    v_player: View<Player>,
    v_creature: View<Creature>,
    mut vm_movement: ViewMut<Movement>,
    mut vm_wpos: ViewMut<WorldPosition>,
) {
    let mut map_pending_updates: Vec<(&ObjectGuid, Position)> = Vec::new();

    for (guid, mut movement) in (&v_guid, &mut vm_movement)
        .iter()
        .filter(|(_, mvmt)| mvmt.is_moving())
    {
        let (new_position, spline_state) = movement.update(dt.0);

        let new_position = Position {
            x: new_position.x,
            y: new_position.y,
            z: new_position.z,
            o: 0., // TODO: Figure out orientation
        };

        map_pending_updates.push((&guid.0, new_position));

        if spline_state == MovementSplineState::Arrived {
            movement.reset_spline();
        }
    }

    // Update the map out of the loop because we need to borrow View<Movement> and the loop already
    // borrows ViewMut<Movement>
    let v_movement = vm_movement.as_view();
    for (guid, pos) in map_pending_updates {
        map.0.update_entity_position(
            guid,
            None, // FIXME: Must be defined if it's a server-controlled player (feared for example)
            &pos,
            &v_movement,
            &v_player,
            &v_creature,
            &mut vm_wpos,
        );
    }
}
