//! The wasm fixture's shutdown listener, registered natively: the same answers as `real_member`.
#[path = "support/no_hooks.rs"]
mod no_hooks;
#[path = "fixture/src/shutdown.rs"]
mod shutdown;

/// `horizon_client::game::server_lifecycle::remember_shutdown`, as the staging pass marks it.
#[horizon_mod_api_macros::point(name = "game_server_lifecycle_remember_shutdown", free)]
fn remember_shutdown(failure: &mut Option<String>, message: String) {
    if failure.is_none() {
        *failure = Some(message);
    }
}

#[test]
fn the_shared_listener_source_answers_natively_as_in_wasm() {
    // Without a listener the body runs as written.
    let mut failure = None;
    remember_shutdown(&mut failure, "disk full".into());
    assert_eq!(failure.as_deref(), Some("disk full"));
    mod_points::register(
        &mod_points::listeners::game_server_lifecycle_remember_shutdown(
            "native",
            0,
            "shutdown",
            shutdown::on_shutdown,
        )
        .ordered(0, "shutdown"),
    )
    .unwrap();
    mod_points::__private::seal();
    for (before, message, after) in shutdown::CASES {
        let mut failure = before.map(String::from);
        remember_shutdown(&mut failure, message.into());
        assert_eq!(failure.as_deref(), after, "{message}");
    }
}
