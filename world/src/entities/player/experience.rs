use crate::{
    entities::object_guid::ObjectGuid,
    protocol::{
        packets::{SmsgLevelUpInfo, SmsgLogXpGain},
        server::ServerMessage,
    },
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

        let packet = ServerMessage::new(SmsgLevelUpInfo::build(next_level));
        self.session.send(&packet).unwrap();
    }
}
