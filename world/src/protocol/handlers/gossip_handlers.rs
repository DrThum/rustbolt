use binrw::NullString;
use log::{error, warn};
use shipyard::{Get, View};

use crate::entities::creature::Creature;
use crate::entities::player::Player;
use crate::game::gossip::GossipMenu;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::{OpcodeHandler, PacketHandlerArgs};
use crate::shared::constants::{CharacterClass, GossipMenuOptionType, TrainerType};

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_gossip_hello(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgGossipHello = ClientMessage::read_as(data).unwrap();

        OpcodeHandler::send_initial_gossip_menu(cmsg.guid, session.clone(), world_context.clone());
    }

    pub fn handle_cmsg_gossip_select_option(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgGossipSelectOption = ClientMessage::read_as(data).unwrap();

        let Some(map) = session.current_map() else {
            error!("handle_cmsg_gossip_select_option: session has no map");
            return;
        };

        let Some(target_entity_id) = map.lookup_entity_ecs(&cmsg.guid) else {
            error!(
                "handle_cmsg_gossip_select_option: map has no EntityId for cmsg.guid (guid: {:?})",
                cmsg.guid
            );
            return;
        };

        let Ok(creature_template) = map.world().run(|v_creature: View<Creature>| {
            v_creature.get(target_entity_id).map(|c| c.template.clone())
        }) else {
            warn!("handle_cmsg_gossip_select_option: target is not a creature, TODO!");
            return;
        };

        let Some(gossip_menu_record) = world_context.data_store.get_gossip_menu(cmsg.menu_id)
        else {
            error!("handle_cmsg_gossip_select_option: received a non-existing menu_id");
            return;
        };

        let gossip_menu = GossipMenu::from_db_record(gossip_menu_record);

        if cmsg.option_index as usize >= gossip_menu.items.len() {
            error!("handle_cmsg_gossip_select_option: received a non-existing option_id (index {} but menu only has {} items)", cmsg.option_index, gossip_menu.items.len());
            return;
        }

        let gossip_menu_option = &gossip_menu_record.options[cmsg.option_index as usize];

        match gossip_menu_option.option_type {
            GossipMenuOptionType::Innkeeper => {
                session.close_gossip_menu();
                let packet = ServerMessage::new(SmsgBinderConfirm {
                    guid: cmsg.guid,
                });
                session.send(&packet).unwrap();
            },
            GossipMenuOptionType::Trainer => {
                let Some(trainer_type) = creature_template.trainer_type else {
                    error!("handle_cmsg_gossip_select_option: received a trainer option but creature is not a trainer");
                    session.close_gossip_menu();
                    return;
                };

                let Some(player_entity_id) = session.player_entity_id() else {
                    error!("handle_cmsg_gossip_select_option: no player_entity_id in session");
                    return;
                };

                let is_valid_trainer = map.world().run(|v_player: View<Player>| {
                    let Ok(player) = v_player.get(player_entity_id) else { return false; };

                    match trainer_type {
                        TrainerType::Class => {
                            if player.class() != creature_template.trainer_class.unwrap() {
                                let gossip_text_id = match creature_template.trainer_class.unwrap() {
                                    CharacterClass::None => 0,
                                    CharacterClass::Warrior => 4985,
                                    CharacterClass::Paladin => 1635,
                                    CharacterClass::Hunter => 10090,
                                    CharacterClass::Rogue => 4797,
                                    CharacterClass::Priest => 4436,
                                    CharacterClass::Shaman => 5003,
                                    CharacterClass::Mage => 328,
                                    CharacterClass::Warlock => 5836,
                                    CharacterClass::Druid => 4913,
                                };

                                OpcodeHandler::send_gossip_text(&cmsg.guid, gossip_text_id, session.clone());
                                return false;
                            }
                        },
                        _ => {
                            // TODO: same for mount with race, tradeskills and pets for non-hunter (see Mangos' IsTrainerOf second half)
                            warn!("add checks for trainer type {trainer_type:?} if needed");
                        },
                    }

                    true
                });

                if !is_valid_trainer {
                    return;
                }

                let Some(trainer_spells) = world_context.data_store.get_trainer_spells_by_creature_entry(creature_template.entry) else {
                    error!("handle_cmsg_gossip_select_option: received a trainer option but no spells found for entry {}",creature_template.entry);
                    session.close_gossip_menu();
                    return;
                };

                let spells_for_packet: Vec<TrainerSpell> = map.world().run(|v_player: View<Player>| {
                    let Ok(player) = v_player.get(session.player_entity_id().unwrap()) else {
                        return Vec::new();
                    };

                    trainer_spells.into_iter()
                        .filter_map(|spell| {
                            if !player.can_train_spell(spell.spell_id, world_context.clone()) {
                                return None;
                            }

                            let required_level = world_context.data_store.get_skill_required_level_for_player(spell.spell_id, player.race_bit(), player.class_bit()).unwrap_or(0);
                            let required_level = required_level.max(spell.required_level);

                            Some(TrainerSpell {
                                spell_id: spell.spell_id,
                                state: spell.state_for_player(player, required_level),
                                cost: spell.spell_cost,
                                can_learn_primary_profession_first_rank: false, // FIXME: professions
                                enable_learn_primary_profession_button: false, // FIXME: professions
                                required_level: required_level as u8,
                                required_skill: spell.required_skill,
                                required_skill_value: spell.required_skill_value,
                                previous_spell: 0, // FIXME: spell chains
                                required_required_spell: 0, // FIXME: spell chains
                                unk: 0, // always 0 in MaNGOS
                            })
                        })
                        .collect()
                });

                let packet = ServerMessage::new(SmsgTrainerList {
                    trainer_guid: cmsg.guid,
                    trainer_type: trainer_type as u32,
                    spell_count: spells_for_packet.len() as u32,
                    spells: spells_for_packet,
                    title: NullString::from("Hello! Ready for some training?"),
                });
                session.send(&packet).unwrap();
            }
            ot => warn!("handle_cmsg_gossip_select_option: received a non-implemented-yet option type {ot:?}"),
        };
    }
}
