use nostr_sdk::{Client, Event, Filter};

pub async fn list_events(client: Client, filter: Filter) -> Result<Vec<Event>, String> {
    // let authors: Option<Vec<XOnlyPublicKey>> = sub_command_args.authors.as_ref().map(|auths| {
    //     auths
    //         .iter()
    //         .map(|a| XOnlyPublicKey::from_str(a.as_str()).expect("Invalid public key"))
    //         .collect()
    // });

    // let kinds: Option<Vec<Kind>> = sub_command_args
    //     .kinds
    //     .as_ref()
    //     .map(|kinds| kinds.iter().map(|k| Kind::from(*k)).collect());

    // let events: Option<Vec<EventId>> = sub_command_args.e.as_ref().map(|events| {
    //     events
    //         .iter()
    //         .map(|e| EventId::from_hex(e.as_str()).expect("Invalid event id"))
    //         .collect()
    // });

    // let pubkeys: Option<Vec<XOnlyPublicKey>> = sub_command_args.p.as_ref().map(|pubs| {
    //     pubs.iter()
    //         .map(|p| XOnlyPublicKey::from_str(p.as_str()).expect("Invalid public key"))
    //         .collect()
    // });

    // Filter {
    //     ids: sub_command_args.ids.clone(),
    //     authors,
    //     kinds,
    //     events,
    //     pubkeys,
    //     hashtags: None,
    //     references: None,
    //     search: None,
    //     since: sub_command_args.since.map(Timestamp::from),
    //     until: sub_command_args.until.map(Timestamp::from),
    //     limit: sub_command_args.limit,
    //     custom: Map::new(),
    // }

    let events: Vec<Event> = client
        .get_events_of(vec![filter], None)
        .await
        .map_err(|e| e.to_string())?;

    Ok(events)
}
