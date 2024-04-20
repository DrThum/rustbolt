#[derive(Debug)]
pub enum RepositoryError {
    DatabaseError(rusqlite::Error),
}

impl From<rusqlite::Error> for RepositoryError {
    fn from(error: rusqlite::Error) -> RepositoryError {
        RepositoryError::DatabaseError(error)
    }
}

pub type RepositoryResult<T> = Result<T, RepositoryError>;
pub type RResult<T> = RepositoryResult<T>;
