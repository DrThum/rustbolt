CREATE TABLE character_quests(
  character_guid INTEGER NOT NULL,
  quest_id INTEGER NOT NULL,
  status INTEGER NOT NULL,
  FOREIGN KEY(character_guid) REFERENCES characters(guid) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_character_quests_quest_id ON character_quests(character_guid, quest_id);
