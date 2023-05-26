CREATE TABLE character_action_buttons(
  character_guid INTEGER NOT NULL,
  position INTEGER NOT NULL,
  action_type INTEGER NOT NULL,
  action_value INTEGER NOT NULL,
  FOREIGN KEY (character_guid) REFERENCES characters(guid) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_character_action_buttons_position ON character_action_buttons(character_guid, position);
