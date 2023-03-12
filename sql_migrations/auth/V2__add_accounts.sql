CREATE TABLE accounts (
  id INTEGER PRIMARY KEY,
  username TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  session_key TEXT NOT NULL
);

INSERT INTO accounts(id, username, password_hash, session_key) VALUES
(NULL, 'a', 'f3f8cd726dc80503b80218793ea0b5f2ab962612', '');
