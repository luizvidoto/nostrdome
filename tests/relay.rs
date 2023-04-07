// use sqlx::{Connection, Executor, SqliteConnection};
// use crate::types::{RelayUrl};
// use crate::relay::DbRelay;
// use anyhow::Result;

// mod setup;

// async fn setup_test_db() -> Result<SqliteConnection> {
//     let pool = setup::setup_db().await?;
//     let mut conn = pool.acquire().await?;
//     setup::create_tables(&mut conn).await?;
//     Ok(conn)
// }

// #[tokio::test]
// async fn test_relay_insert_fetch() -> Result<()> {
//     let mut conn = setup_test_db().await?;
//     // Create a relay URL
//     let relay_url = RelayUrl::try_from_str("https://relay.example.com").unwrap();

//     let db_relay = DbRelay {
//         url: relay_url.clone(),
//         last_connected_at: Some(1627551116),
//         read: true,
//         write: true,
//         advertise: false,
//     };

//     DbRelay::insert(&conn, db_relay.clone()).await?;

//     let fetched_db_relay = DbRelay::fetch_one(&conn, &relay_url).await?.unwrap();
//     assert_eq!(db_relay, fetched_db_relay);

//     Ok(())
// }
