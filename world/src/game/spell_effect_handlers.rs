use std::sync::Arc;

use log::warn;
use shipyard::{AllStoragesViewMut, Get, View, ViewMut};

use crate::{
    datastore::data_types::{GameObjectData, MapRecord, SpellRecord},
    ecs::components::{guid::Guid, powers::Powers, threat_list::ThreatList, unit::Unit},
    entities::{creature::Creature, game_object::GameObject, player::Player},
    protocol::{
        packets::{LootResponseItem, SmsgLootResponse},
        server::ServerMessage,
    },
    shared::constants::{LootSlotType, LootType, UnitDynamicFlag, UnitFlags},
};

use super::{
    experience::Experience, spell::Spell, spell_effect_handler::SpellEffectHandler,
    world_context::WorldContext,
};

impl SpellEffectHandler {
    pub(crate) fn unhandled(
        _world_context: Arc<WorldContext>,
        _spell: Arc<Spell>,
        _map_record: &MapRecord,
        _spell_record: Arc<SpellRecord>,
        _effect_index: usize,
        _all_storages: &AllStoragesViewMut,
    ) {
    }

    pub fn handle_effect_school_damage(
        _world_context: Arc<WorldContext>,
        spell: Arc<Spell>,
        map_record: &MapRecord,
        spell_record: Arc<SpellRecord>,
        effect_index: usize,
        all_storages: &AllStoragesViewMut,
    ) {
        all_storages.run(
            |mut vm_powers: ViewMut<Powers>,
             mut vm_threat_list: ViewMut<ThreatList>,
             v_guid: View<Guid>,
             mut vm_player: ViewMut<Player>,
             v_creature: View<Creature>,
             v_unit: View<Unit>| {
                let Some(unit_target) = spell.unit_target() else {
                    warn!("handle_effect_school_damage: no unit target");
                    return;
                };

                let damage = spell_record.calc_simple_value(effect_index);
                let target_powers = &mut vm_powers[unit_target];
                target_powers.apply_damage(damage as u32);
                // TODO: Log damage somehow

                if target_powers.is_alive() {
                    if let Ok(mut threat_list) = (&mut vm_threat_list).get(unit_target) {
                        threat_list.modify_threat(spell.caster(), damage as f32);
                    }
                } else if let Ok(mut player) = (&mut vm_player).get(spell.caster()) {
                    // FIXME: This logic is duplicated in melee.rs
                    let target_guid = v_guid[unit_target].0;
                    let mut has_loot = false; // TODO: Handle player case (Insignia looting in PvP)
                    if let Ok(creature) = v_creature.get(unit_target) {
                        let xp_gain = Experience::xp_gain_against(&player, creature, map_record);
                        player.give_experience(xp_gain, Some(target_guid));
                        player.notify_killed_creature(creature.guid(), creature.template.entry);

                        has_loot = creature.generate_loot();
                    }

                    if let Ok(target_unit) = v_unit.get(unit_target) {
                        if has_loot {
                            target_unit.set_dynamic_flag(UnitDynamicFlag::Lootable);
                        }
                    }

                    player.unset_in_combat_with(target_guid);
                }
            },
        );
    }

    pub fn handle_effect_heal(
        _world_context: Arc<WorldContext>,
        spell: Arc<Spell>,
        _map_record: &MapRecord,
        _spell_record: Arc<SpellRecord>,
        effect_index: usize,
        all_storages: &AllStoragesViewMut,
    ) {
        all_storages.run(|mut vm_powers: ViewMut<Powers>| {
            let Some(unit_target) = spell.unit_target() else {
                warn!("handle_effect_school_damage: no unit target");
                return;
            };

            let damage = _spell_record.calc_simple_value(effect_index);
            vm_powers[unit_target].apply_healing(damage as u32);
        });
    }

    pub fn handle_effect_open_lock(
        world_context: Arc<WorldContext>,
        spell: Arc<Spell>,
        _map_record: &MapRecord,
        _spell_record: Arc<SpellRecord>,
        _effect_index: usize,
        all_storages: &AllStoragesViewMut,
    ) {
        all_storages.run(
            |v_game_object: View<GameObject>,
             v_unit: View<Unit>,
             mut vm_player: ViewMut<Player>| {
                let Some(game_object_target) = spell.game_object_target() else {
                    warn!("spell effect OpenLock: no game object target");
                    return;
                };

                if let Ok(game_object) = v_game_object.get(game_object_target) {
                    let player = &mut vm_player[spell.caster()];
                    // TODO: Check that the player can open this lock (CanOpenLock in MaNGOS)

                    v_unit[spell.caster()].set_unit_flag(UnitFlags::Looting);
                    player.set_looting(spell.game_object_target());

                    game_object.generate_loot(false);

                    let loot_items: Vec<LootResponseItem> = game_object
                        .loot()
                        .items()
                        .iter()
                        .map(|li| {
                            if let Some(item_template) =
                                world_context.data_store.get_item_template(li.item_id)
                            {
                                LootResponseItem {
                                    index: li.index,
                                    id: li.item_id,
                                    count: li.count,
                                    display_info_id: item_template.display_id,
                                    random_suffix: li.random_suffix,
                                    random_property_id: li.random_property_id,
                                    slot_type: LootSlotType::Normal,
                                }
                            } else {
                                panic!("found non-existing item when generating creature loot");
                            }
                        })
                        .collect();

                    let packet = ServerMessage::new(SmsgLootResponse::build(
                        &game_object.guid(),
                        LootType::Pickpocketing,
                        0,
                        loot_items,
                    ));
                    player.session.send(&packet).unwrap();

                    // Type-specific handling
                    #[allow(clippy::single_match)] // More types to come later
                    match game_object.data {
                        GameObjectData::Goober { .. } => player.notify_interacted_with_game_object(
                            &game_object.guid(),
                            game_object.entry,
                        ),
                        _ => (),
                    }

                    // TODO: Increase this lock's skill (end of EffectOpenLock in MaNGOS)
                }
            },
        );
    }
}
