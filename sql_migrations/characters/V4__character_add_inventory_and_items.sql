CREATE TABLE items (
  guid INTEGER PRIMARY KEY,
  entry INTEGER NOT NULL
);

CREATE TABLE character_inventory (
  character_guid INTEGER NOT NULL,
  item_guid INTEGER NOT NULL,
  slot INTEGER NOT NULL,
  FOREIGN KEY(character_guid) REFERENCES characters(guid) ON DELETE CASCADE,
  FOREIGN KEY(item_guid) REFERENCES items(guid) ON DELETE CASCADE,
  UNIQUE(character_guid, slot)
)
