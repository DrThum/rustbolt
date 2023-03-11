CREATE TABLE realms (
  id INTEGER PRIMARY KEY,
  realm_type INTEGER NOT NULL,
  is_locked INTEGER NOT NULL,
  flags INTEGER NOT NULL,
  name TEXT NOT NULL,
  address TEXT NOT NULL,
  population REAL NOT NULL,
  category INTEGER NOT NULL
);

INSERT INTO realms(id, realm_type, is_locked, flags, name, address, population, category) VALUES
(NULL, 1, FALSE, 0x20, 'Rustbolt', '127.0.0.1:8085', 200.0, 10);
