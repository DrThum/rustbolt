CREATE TABLE character_spell_cooldowns(
  character_guid INTEGER NOT NULL,
  spell_id INTEGER NOT NULL,
  item_id INTEGER,
  cooldown_end_timestamp INTEGER NOT NULL,
  FOREIGN KEY(character_guid) REFERENCES characters(guid) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_character_spell_cooldowns_spell_id ON character_spell_cooldowns(character_guid, spell_id);

