use mod_api::{
    Hooks, IPayloadContext, IPayloadHandler, ModEntry, PayloadBytes, PayloadDirection, PayloadId,
    PayloadProtocol, Registrar,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

const NAME: &str =
    "game::payload_transport::tests::integrated_memory_delivers_registered_raw_payload_and_reply";
const CHANNEL: &str = "raw_probe:message";
struct Probe {
    expected: Arc<Vec<u8>>,
    server: Arc<AtomicUsize>,
    client: Arc<AtomicUsize>,
}
struct Handler {
    expected: Arc<Vec<u8>>,
    hits: Arc<AtomicUsize>,
    server: bool,
}
impl IPayloadHandler for Handler {
    fn handle(&self, bytes: PayloadBytes, context: &mut dyn IPayloadContext) -> Result<(), String> {
        assert_eq!(
            &bytes.0,
            self.expected.as_ref(),
            "exact raw application bytes"
        );
        assert_eq!(context.protocol(), PayloadProtocol::Play);
        assert!(
            context.player().is_ok(),
            "a live connection supplies the player"
        );
        if self.server {
            assert_eq!(std::thread::current().name(), Some("Server thread"));
            assert_eq!(context.flow(), PayloadDirection::ToServer);
            assert!(context.server_host().is_ok());
            assert!(context.client_host().is_err());
            context
                .reply(&PayloadId(CHANNEL.into()), bytes)
                .map_err(|e| format!("reply: {e:?}"))?;
        } else {
            assert_eq!(context.flow(), PayloadDirection::ToClient);
            assert!(context.client_host().is_ok());
            assert!(context.server_host().is_err());
        }
        self.hits.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
#[mod_api::__subscriptions]
impl Hooks for Probe {
    fn register(&self, registrar: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
        registrar
            .payload_handlers()
            .registrar("1")
            .play_bidirectional(
                PayloadId(CHANNEL.into()),
                Arc::new(Handler {
                    expected: self.expected.clone(),
                    hits: self.server.clone(),
                    server: true,
                }),
                Some(Arc::new(Handler {
                    expected: self.expected.clone(),
                    hits: self.client.clone(),
                    server: false,
                })),
            );
    }
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES; starts a real integrated server"]
fn integrated_memory_delivers_registered_raw_payload_and_reply() {
    if super::super::native_entity_refs::tests::run_fresh_cases(
        NAME,
        "HORIZON_RAW_TRANSPORT_CHILD",
        &["one_mib"],
    ) {
        return;
    }
    let expected = Arc::new(
        (0..1024 * 1024)
            .map(|i| ((i * 31 + 7) % 251) as u8)
            .collect::<Vec<_>>(),
    );
    let server = Arc::new(AtomicUsize::new(0));
    let client = Arc::new(AtomicUsize::new(0));
    let mods = vec![ModEntry::new(
        "raw_probe",
        Box::new(Probe {
            expected: expected.clone(),
            server: server.clone(),
            client: client.clone(),
        }),
    )]
    .into();
    mod_api::testing::with_mods(mods, || {
        super::super::native_entity_refs::tests::with_client_game(|game| {
            game.settings.render_distance = 2;
            game.settings.simulation_distance = 2;
            let options = horizon_client::title::WorldOptions {
                world_type: horizon_client::save::WorldPreset::Flat,
                ..Default::default()
            };
            game.open_integrated(
                "raw_transport",
                Some("Raw transport"),
                Some(1),
                true,
                Some(&options),
            )
            .unwrap();
            let deadline = Instant::now() + Duration::from_secs(60);
            loop {
                game.poll_remote().unwrap();
                if game
                    .remote
                    .as_ref()
                    .is_some_and(|remote| remote.logged_in && remote.joined)
                {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "integrated play did not become ready"
                );
                std::thread::yield_now();
            }
            assert!(
                game.integrated.is_some(),
                "production integrated server handle"
            );
            let host: &mut dyn mod_api::ClientHost = game;
            assert_eq!(
                host.send_payload_to_server(
                    &PayloadId(CHANNEL.into()),
                    PayloadBytes(expected.as_ref().clone())
                ),
                Ok(mod_api::PayloadAccepted::Queued)
            );
            while client.load(Ordering::SeqCst) == 0 {
                game.poll_remote().unwrap();
                assert!(
                    Instant::now() < deadline,
                    "registered round trip did not complete; server hits {}",
                    server.load(Ordering::SeqCst)
                );
                std::thread::yield_now();
            }
            assert_eq!(server.load(Ordering::SeqCst), 1);
            assert_eq!(client.load(Ordering::SeqCst), 1);
            game.leave_server(None).unwrap();
        });
    });
}
