use sqlx::{sqlite::SqliteRow, Row};

#[derive(Debug, Clone)]
pub struct DbUser {
    pub pub_key: String,
    pub name: String,
}
impl DbUser {
    const _FETCH_QUERY: &'static str = "SELECT pub_key, name FROM user";

    // Funções CRUD (create, read, update, delete) para DbUser
    // fetch, fetch_one, insert, update, delete
}

impl sqlx::FromRow<'_, SqliteRow> for DbUser {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(DbUser {
            pub_key: row.try_get::<String, &str>("pub_key")?,
            name: row.try_get::<String, &str>("name")?,
        })
    }
}
