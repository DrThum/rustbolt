use shipyard::{IntoIter, UniqueView, View};

use crate::{
    ecs::components::guid::Guid,
    entities::{
        internal_values::WrappedInternalValues,
        position::WorldPosition,
        update::{UpdateBlockBuilder, UpdateData, UpdateType},
    },
    game::map_manager::WrappedMapManager,
    protocol::packets::SmsgUpdateObject,
};

pub fn send_entity_update(
    map_manager: UniqueView<WrappedMapManager>,
    v_guid: View<Guid>,
    v_int_vals: View<WrappedInternalValues>,
    v_wpos: View<WorldPosition>,
) {
    for (guid, wrapped_int_vals, wpos) in (&v_guid, &v_int_vals, &v_wpos).iter() {
        let mut internal_values = wrapped_int_vals.0.write();
        if internal_values.has_dirty() {
            for session in map_manager.0.nearby_sessions(wpos) {
                let mut update_builder = UpdateBlockBuilder::new();

                for index in internal_values.get_dirty_indexes() {
                    let value = internal_values.get_u32(index as usize);
                    update_builder.add(index as usize, value);
                }

                let blocks = update_builder.build();

                let update_data = vec![UpdateData {
                    update_type: UpdateType::Values,
                    packed_guid: guid.0.as_packed(),
                    blocks,
                }];

                let smsg_update_object = SmsgUpdateObject {
                    updates_count: update_data.len() as u32,
                    has_transport: false,
                    updates: update_data,
                };

                session.update_entity(smsg_update_object);
            }

            internal_values.reset_dirty();
        }
    }
}
