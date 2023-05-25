CREATE TABLE character_skills(
  character_guid INTEGER NOT NULL,
  skill_id INTEGER NOT NULL,
  value INTEGER NOT NULL,
  max_value INTEGER NOT NULL,
  FOREIGN KEY(character_guid) REFERENCES characters(guid) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_character_skills_skill_id ON character_skills(character_guid, skill_id);
