use shipyard::{AllStoragesViewMut, Get, IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    ecs::components::applied_auras::AppliedAuras,
    entities::player::Player,
    game::{map::HasPlayers, world_context::WrappedWorldContext},
};

pub fn update_auras(vm_all_storages: AllStoragesViewMut) {
    vm_all_storages.run(
        |has_players: UniqueView<HasPlayers>,
         world_context: UniqueView<WrappedWorldContext>,
         v_player: View<Player>,
         mut vm_app_auras: ViewMut<AppliedAuras>| {
            if !**has_players {
                return;
            }

            for (entity_id, applied_auras) in (&mut vm_app_auras).iter().with_id() {
                let player = v_player.get(entity_id).ok();
                let session = player.map(|p| p.session.clone());

                applied_auras.update(session, world_context.clone(), &vm_all_storages);
            }
        },
    )
}
