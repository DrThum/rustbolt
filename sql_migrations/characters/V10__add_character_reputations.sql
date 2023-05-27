CREATE TABLE character_reputations(
  character_guid INTEGER NOT NULL,
  faction_id INTEGER NOT NULL,
  standing INTEGER NOT NULL,
  flags INTEGER NOT NULL,
  FOREIGN KEY(character_guid) REFERENCES characters(guid) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_character_reputations_faction_id ON character_reputations(character_guid, faction_id);
