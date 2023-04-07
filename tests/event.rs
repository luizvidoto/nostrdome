// use crate::event::DbEvent;
// use crate::types::{Event, EventId, Kind, Timestamp, XOnlyPublicKey};
// use anyhow::Result;
// use rand::Rng;
// use sqlx::{Connection, Executor, SqliteConnection};

// mod setup;

// async fn setup_test_db() -> Result<SqliteConnection> {
//     let pool = setup::setup_db().await?;
//     let mut conn = pool.acquire().await?;
//     setup::create_tables(&mut conn).await?;
//     Ok(conn)
// }

// #[tokio::test]
// async fn test_event_insert_fetch() -> Result<()> {
//     let mut conn = setup_test_db().await?;
//     // Create a random event
//     let event = Event {
//         id: EventId(rand::thread_rng().gen::<u64>()),
//         pubkey: XOnlyPublicKey::from_slice(&rand::thread_rng().gen::<[u8; 32]>()).unwrap(),
//         created_at: Timestamp::now(),
//         kind: Kind::Text,
//         tags: vec![],
//         content: "Test content".to_owned(),
//         sig: Signature::from_compact(&rand::thread_rng().gen::<[u8; 64]>()).unwrap(),
//         ots: Some("OpenTimestamps data".to_owned()),
//     };

//     let db_event = DbEvent::new(event.clone());
//     DbEvent::insert(&conn, db_event.clone()).await?;

//     let fetched_db_event = DbEvent::fetch_one(&conn, &event.id).await?.unwrap();
//     assert_eq!(db_event, fetched_db_event);

//     Ok(())
// }
