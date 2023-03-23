use rusqlite::Connection;

pub struct AccountRepository;

impl AccountRepository {
    pub fn fetch_id_and_session_key(
        conn: &mut Connection,
        username: String,
    ) -> Option<(u32, String)> {
        let mut stmt = conn
            .prepare("SELECT id, session_key FROM accounts WHERE UPPER(username) = :username")
            .unwrap();
        let mut rows = stmt.query(&[(":username", &username)]).unwrap();

        if let Some(row) = rows.next().unwrap() {
            Some((row.get("id").unwrap(), row.get("session_key").unwrap()))
        } else {
            None
        }
    }
}
