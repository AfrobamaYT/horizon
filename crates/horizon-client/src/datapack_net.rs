//! The registries of mods' datapack registries on the wire (`horizon:datapack_registries`,
//! arismoko/horizon#811, stage 4): at the join, a Horizon client offers the synchronized
//! registries it declared, with their schemas (`horizon:datapack_offer`), and the server answers
//! with a snapshot of the ones both sides have, before any registry data. A required registry
//! the client did not offer ends the join with a reason; an optional one is left out and
//! nothing is sent in its place. The client checks the snapshot against its own declarations
//! before it keeps anything, and a snapshot that fails ends the join. The set a server last
//! published and the set a client received are held here, for the lan thread and the game.
//!
//! Only the join carries a snapshot so far: a `/reload` publishes the next generation on the
//! server and clients keep the one they joined with until they rejoin. See
//! `docs/modding/src/concepts/mod-datapack-registries.md`.
use std::sync::{Arc, Mutex, atomic::AtomicU64};

/// The snapshot, server to client.
pub const CHANNEL: &str = "horizon:datapack_registries";
/// The client's offer, client to server.
pub const OFFER_CHANNEL: &str = "horizon:datapack_offer";

type Held = Mutex<Option<Arc<mod_api::DatapackRegistrySet>>>;
/// What this process's server last published.
static SERVED: Held = Mutex::new(None);
/// What this process's client last received from a server.
static RECEIVED: Held = Mutex::new(None);
static SESSION: AtomicU64 = AtomicU64::new(1);

fn held(slot: &Held) -> Option<Arc<mod_api::DatapackRegistrySet>> {
    slot.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn put(slot: &Held, set: Option<Arc<mod_api::DatapackRegistrySet>>) {
    *slot.lock().unwrap_or_else(|e| e.into_inner()) = set;
}

/// Keeps the set a server just published, for the clients that join it.
pub fn serve(set: Option<Arc<mod_api::DatapackRegistrySet>>) {
    put(&SERVED, set);
}

/// The set this process's server last published.
pub fn served() -> Option<Arc<mod_api::DatapackRegistrySet>> {
    held(&SERVED)
}

/// The set this process's client received from the server it joined, if the server sent one.
pub fn received() -> Option<Arc<mod_api::DatapackRegistrySet>> {
    held(&RECEIVED)
}

/// Forgets what an earlier join received, as a new join starts.
pub fn clear_received() {
    put(&RECEIVED, None);
}

/// What the client sends its server: its synchronized registries and their schemas, or `None`
/// when it declared none.
pub fn offer() -> Option<Vec<u8>> {
    let declarations = mod_api::__private::datapack_registry_declarations();
    declarations
        .iter()
        .any(|declaration| declaration.sync != mod_api::DatapackRegistrySync::ServerOnly)
        .then(|| mod_api::DatapackSnapshot::encode_offer(declarations))
        .flatten()
}

/// What the server does for a joining client.
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    /// Nothing: no mod declared a synchronized registry, or none applies to this client.
    Nothing,
    /// Send these bytes on [`CHANNEL`], before the registry data.
    Send(Vec<u8>),
    /// End the join with this reason, before any payload.
    Refuse(String),
}

/// The server's step for a client of the given brand, with the offer it sent if it sent one.
pub fn server_step(horizon_client: bool, offer: Option<&[u8]>) -> Step {
    let declarations = mod_api::__private::datapack_registry_declarations();
    step_for(declarations, served().as_deref(), horizon_client, offer)
}

fn step_for(
    declarations: &[mod_api::DatapackRegistryDeclaration],
    set: Option<&mod_api::DatapackRegistrySet>,
    horizon_client: bool,
    offer: Option<&[u8]>,
) -> Step {
    use mod_api::DatapackRegistrySync;
    if declarations
        .iter()
        .all(|declaration| declaration.sync == DatapackRegistrySync::ServerOnly)
    {
        return Step::Nothing;
    }
    let Some(set) = set else {
        return Step::Nothing;
    };
    // A vanilla client, or a Horizon one that sent no offer, has no registries to match.
    let offered = match (horizon_client, offer) {
        (true, Some(bytes)) => match mod_api::DatapackSnapshot::decode_offer(bytes) {
            Ok(offered) => offered,
            Err(_) => return Step::Refuse("The client's mod registries are unreadable.".into()),
        },
        _ => Vec::new(),
    };
    match set.snapshot_for(declarations, &offered) {
        Err(mod_api::DatapackNegotiationError::RequiredRegistry { registry, mod_id }) => {
            Step::Refuse(format!(
                "Install the mod {mod_id} of the server: it needs the registry {registry}."
            ))
        }
        Err(_) => Step::Refuse("The client's mod registries are unreadable.".into()),
        Ok(snapshot) if snapshot.registries.is_empty() => Step::Nothing,
        Ok(snapshot) => match snapshot.encode() {
            Some(bytes) => Step::Send(bytes),
            None => Step::Refuse("The server's mod registries are too large to send.".into()),
        },
    }
}

/// The client's side of a snapshot: checks it against what the client declared and keeps it as
/// the set its mods read, or says why the join must end. Nothing is kept on failure.
pub fn accept(bytes: &[u8]) -> Result<(), String> {
    let declarations = mod_api::__private::datapack_registry_declarations();
    let received = receive(declarations, bytes)?;
    put(&RECEIVED, Some(Arc::new(received)));
    Ok(())
}

fn receive(
    declarations: &[mod_api::DatapackRegistryDeclaration],
    bytes: &[u8],
) -> Result<mod_api::DatapackRegistrySet, String> {
    let refused = |error: mod_api::DatapackSnapshotError| {
        format!("The server's mod registries do not match this client's: {error:?}")
    };
    let snapshot = mod_api::DatapackSnapshot::decode(bytes).map_err(refused)?;
    let candidate = snapshot.candidate(declarations).map_err(refused)?;
    let session = SESSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    Ok(mod_api::DatapackRegistrySet::received(
        &snapshot, candidate, session,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mod_api::{
        DatapackCandidate, DatapackField, DatapackFieldKind, DatapackRegistryDeclaration,
        DatapackRegistrySet, DatapackRegistrySync,
    };

    fn spell(sync: DatapackRegistrySync) -> DatapackRegistryDeclaration {
        DatapackRegistryDeclaration {
            mod_id: "example".into(),
            key: "example:spell".into(),
            fields: vec![DatapackField::required(
                "power",
                DatapackFieldKind::Int { min: 0, max: 100 },
            )],
            sync,
        }
    }

    fn set(declaration: &DatapackRegistryDeclaration) -> DatapackRegistrySet {
        let files = vec![(
            "data/addon/example/spell/fire.json".to_owned(),
            vec![br#"{"power": 7}"#.to_vec()],
        )];
        DatapackRegistrySet::first(
            DatapackCandidate::build(std::slice::from_ref(declaration), &files).unwrap(),
            1,
        )
    }

    #[test]
    fn a_client_with_the_mod_receives_the_registry_and_reads_it() {
        let declaration = spell(DatapackRegistrySync::Required);
        let declarations = std::slice::from_ref(&declaration);
        let server = set(&declaration);
        let offer = mod_api::DatapackSnapshot::encode_offer(declarations).unwrap();
        let Step::Send(bytes) = step_for(declarations, Some(&server), true, Some(&offer)) else {
            panic!("a snapshot is sent");
        };
        let client = receive(declarations, &bytes).unwrap();
        let handle = client.handle("example:spell", "addon:fire").unwrap();
        assert_eq!(
            client.get(&handle).unwrap().get("power"),
            Some(&mod_api::DatapackValue::Int(7))
        );
        // Truncated bytes are refused and nothing is kept.
        assert!(receive(declarations, &bytes[..bytes.len() - 1]).is_err());
    }

    #[test]
    fn a_required_registry_refuses_a_client_that_lacks_it() {
        let declaration = spell(DatapackRegistrySync::Required);
        let declarations = std::slice::from_ref(&declaration);
        let server = set(&declaration);
        for (horizon, offer) in [(false, None), (true, None)] {
            let Step::Refuse(reason) = step_for(declarations, Some(&server), horizon, offer) else {
                panic!("the join is refused");
            };
            assert!(reason.contains("example") && reason.contains("example:spell"));
        }
        // Bytes that are not an offer are refused too.
        assert!(matches!(
            step_for(declarations, Some(&server), true, Some(&[1, 2, 3])),
            Step::Refuse(_)
        ));
    }

    #[test]
    fn optional_and_server_only_registries_never_end_a_join() {
        let optional = spell(DatapackRegistrySync::Optional);
        assert_eq!(
            step_for(
                std::slice::from_ref(&optional),
                Some(&set(&optional)),
                false,
                None
            ),
            Step::Nothing
        );
        let only = spell(DatapackRegistrySync::ServerOnly);
        assert_eq!(
            step_for(std::slice::from_ref(&only), Some(&set(&only)), true, None),
            Step::Nothing
        );
        // No world published yet: nothing to send.
        assert_eq!(
            step_for(std::slice::from_ref(&optional), None, true, None),
            Step::Nothing
        );
    }
}
