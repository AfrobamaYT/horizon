use horizon_client::lan::hub::HostEvent;

/// What a hub event is, whose connection, and the name of a player who left (no longer in the
/// hub once it is handled): for a slow tick's notes.
pub(super) type EventKind = (&'static str, Option<u32>, Option<String>);

pub(super) fn event_kind(event: &HostEvent) -> EventKind {
    match event {
        HostEvent::Joined(c) => ("join", Some(*c), None),
        HostEvent::Left(g) => ("leave", None, Some(g.name.clone())),
        HostEvent::Packet(c, p) => (p.name(), Some(*c), None),
        HostEvent::InputCredits(_) => ("input credits", None, None),
        HostEvent::RawPayload(_) => ("raw payload", None, None),
        HostEvent::ClientInformation(c, _, _) => ("client settings", Some(*c), None),
        HostEvent::ChatSubmitted { .. } => ("chat submitted", None, None),
        HostEvent::ChatSessionStarted(_) => ("chat session", None, None),
        HostEvent::Jump { conn, .. } => ("jump", Some(*conn), None),
        HostEvent::PlayerMove(c) => ("player move", Some(*c), None),
        HostEvent::PlayerLoaded(c) => ("loaded", Some(*c), None),
        HostEvent::RecipeBookSettings { conn, .. } => ("recipe book", Some(*conn), None),
        HostEvent::Sneak { conn, .. } => ("sneak", Some(*conn), None),
        HostEvent::Input { conn, .. } => ("input", Some(*conn), None),
        HostEvent::Kicked { name, .. } => ("kick", None, Some(name.clone())),
        HostEvent::Flight { conn, .. } => ("flight", Some(*conn), None),
        HostEvent::Chat { .. } => ("chat line", None, None),
        HostEvent::PlayerChat { .. } => ("chat", None, None),
        HostEvent::ChatFiltered { name, .. } => ("filtered chat", None, Some(name.clone())),
        HostEvent::Warn(_) => ("warning", None, None),
        HostEvent::Error(_) => ("error", None, None),
        HostEvent::Ban { name, .. } => ("ban", None, Some(name.clone())),
    }
}
