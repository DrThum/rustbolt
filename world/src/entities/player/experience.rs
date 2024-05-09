use crate::{
    entities::object_guid::ObjectGuid,
    protocol::{
        packets::{SmsgLevelUpInfo, SmsgLogXpGain},
        server::ServerMessage,
    },
    shared::constants::{AttributeModifier, AttributeModifierType, CharacterClass, CharacterRace},
};

use super::{Player, UnitFields};

impl Player {
    pub fn experience(&self) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::PlayerXp.into())
    }

    pub fn experience_for_next_level(&self) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::PlayerNextLevelXp.into())
    }

    pub fn give_experience(&self, xp: u32, victim_guid: Option<ObjectGuid>) {
        let packet = ServerMessage::new(SmsgLogXpGain::build(victim_guid, xp));
        let current_xp = self.experience();
        let mut new_xp = current_xp + xp;
        let mut next_level_xp = self.experience_for_next_level();

        while new_xp >= next_level_xp
            && self.level() <= self.world_context.config.world.game.player.maxlevel
        {
            self.increment_level();
            new_xp -= next_level_xp;
            next_level_xp = self.experience_for_next_level();
        }

        self.internal_values
            .write()
            .set_u32(UnitFields::PlayerXp.into(), new_xp);

        self.session.send(&packet).unwrap();
    }

    pub fn level(&self) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldLevel.into())
    }

    pub fn increment_level(&self) {
        let current_level = self.level();
        if current_level >= self.world_context.config.world.game.player.maxlevel {
            return;
        }

        let next_level = current_level + 1;
        let next_level_xp = self
            .world_context
            .data_store
            .get_player_required_experience_at_level(next_level);

        {
            let mut guard = self.internal_values.write();
            guard.set_u32(UnitFields::UnitFieldLevel.into(), next_level);
            guard.set_u32(UnitFields::PlayerNextLevelXp.into(), next_level_xp);
            guard.set_u32(UnitFields::PlayerXp.into(), 0);
        }

        let class = CharacterClass::n(
            self.internal_values
                .read()
                .get_u8(UnitFields::UnitFieldBytes0.into(), 0),
        )
        .unwrap();
        let race = CharacterRace::n(
            self.internal_values
                .read()
                .get_u8(UnitFields::UnitFieldBytes0.into(), 1),
        )
        .unwrap();
        let ds = self.world_context.data_store.clone();
        let current_level_base_health_mana = ds
            .get_player_base_health_mana(class, current_level)
            .expect("base health and mana not found upon level-up");
        let next_level_base_health_mana = ds
            .get_player_base_health_mana(class, next_level)
            .expect("base health and mana not found upon level-up");
        let current_level_base_attributes = ds
            .get_player_base_attributes(race, class, current_level)
            .expect("base attributes not found upon level-up");
        let next_level_base_attributes = ds
            .get_player_base_attributes(race, class, next_level)
            .expect("base attributes not found upon level-up");

        let health_gained =
            next_level_base_health_mana.base_health - current_level_base_health_mana.base_health;
        let mana_gained =
            next_level_base_health_mana.base_mana - current_level_base_health_mana.base_mana;
        let strength_gained =
            next_level_base_attributes.strength - current_level_base_attributes.strength;
        let agility_gained =
            next_level_base_attributes.agility - current_level_base_attributes.agility;
        let stamina_gained =
            next_level_base_attributes.stamina - current_level_base_attributes.stamina;
        let intellect_gained =
            next_level_base_attributes.intellect - current_level_base_attributes.intellect;
        let spirit_gained =
            next_level_base_attributes.spirit - current_level_base_attributes.spirit;

        let packet = ServerMessage::new(SmsgLevelUpInfo::build(
            next_level,
            health_gained,
            mana_gained,
            strength_gained,
            agility_gained,
            stamina_gained,
            intellect_gained,
            spirit_gained,
        ));
        self.session.send(&packet).unwrap();

        {
            let mut attr_mods = self.attribute_modifiers.write();
            attr_mods.add_modifier(
                AttributeModifier::Health,
                AttributeModifierType::BaseValue,
                health_gained as f32,
            );
            attr_mods.add_modifier(
                AttributeModifier::Mana,
                AttributeModifierType::BaseValue,
                mana_gained as f32,
            );
            attr_mods.add_modifier(
                AttributeModifier::StatStrength,
                AttributeModifierType::BaseValue,
                strength_gained as f32,
            );
            attr_mods.add_modifier(
                AttributeModifier::StatAgility,
                AttributeModifierType::BaseValue,
                agility_gained as f32,
            );
            attr_mods.add_modifier(
                AttributeModifier::StatStamina,
                AttributeModifierType::BaseValue,
                stamina_gained as f32,
            );
            attr_mods.add_modifier(
                AttributeModifier::StatIntellect,
                AttributeModifierType::BaseValue,
                intellect_gained as f32,
            );
            attr_mods.add_modifier(
                AttributeModifier::StatSpirit,
                AttributeModifierType::BaseValue,
                spirit_gained as f32,
            );
        }
    }
}
