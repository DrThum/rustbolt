CREATE TABLE character_spells(
  character_guid INTEGER NOT NULL,
  spell_id INTEGER NOT NULL,
  FOREIGN KEY(character_guid) REFERENCES characters(guid) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_character_spells_spell_id ON character_spells(character_guid, spell_id);
