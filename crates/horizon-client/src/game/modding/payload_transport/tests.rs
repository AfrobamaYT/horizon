use mod_api::{
    HandlerThread, Hooks, IPayloadContext, IPayloadHandler, ModEntry, PayloadBytes,
    PayloadContextError, PayloadDirection, PayloadId, PayloadProtocol, PayloadWork,
    PayloadWorkStatus, Registrar,
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
    network: bool,
}
struct Handler {
    expected: Arc<Vec<u8>>,
    hits: Arc<AtomicUsize>,
    server: bool,
    network: bool,
}
struct Work {
    bytes: std::sync::Mutex<Option<PayloadBytes>>,
    hits: Arc<AtomicUsize>,
    server: bool,
    network_thread: std::thread::ThreadId,
}
impl PayloadWork for Work {
    fn run(&self, context: &mut dyn IPayloadContext) -> Result<(), String> {
        assert_ne!(
            std::thread::current().id(),
            self.network_thread,
            "work must actually switch executors"
        );
        assert_eq!(context.protocol(), PayloadProtocol::Play);
        assert!(context.player().is_ok());
        finish(
            self.bytes.lock().unwrap().take().unwrap(),
            context,
            self.server,
            &self.hits,
        )
    }
}
fn finish(
    bytes: PayloadBytes,
    context: &mut dyn IPayloadContext,
    server: bool,
    hits: &AtomicUsize,
) -> Result<(), String> {
    if server {
        assert_eq!(std::thread::current().name(), Some("Server thread"));
        assert_eq!(context.flow(), PayloadDirection::ToServer);
        assert!(context.server_host().is_ok());
        assert!(matches!(
            context.client_host(),
            Err(PayloadContextError::WrongSide)
        ));
        context
            .reply(&PayloadId(CHANNEL.into()), bytes)
            .map_err(|error| format!("reply: {error:?}"))?;
    } else {
        assert_eq!(context.flow(), PayloadDirection::ToClient);
        assert!(context.client_host().is_ok());
        assert!(matches!(
            context.server_host(),
            Err(PayloadContextError::WrongSide)
        ));
    }
    hits.fetch_add(1, Ordering::SeqCst);
    Ok(())
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
        if self.network {
            if self.server {
                assert!(matches!(
                    context.server_host(),
                    Err(PayloadContextError::WrongThread)
                ));
                assert!(matches!(
                    context.client_host(),
                    Err(PayloadContextError::WrongSide)
                ));
            } else {
                assert!(matches!(
                    context.client_host(),
                    Err(PayloadContextError::WrongThread)
                ));
                assert!(matches!(
                    context.server_host(),
                    Err(PayloadContextError::WrongSide)
                ));
            }
            assert_eq!(
                context.enqueue_work(Arc::new(Work {
                    bytes: std::sync::Mutex::new(Some(bytes)),
                    hits: self.hits.clone(),
                    server: self.server,
                    network_thread: std::thread::current().id(),
                })),
                Ok(PayloadWorkStatus::Queued)
            );
            Ok(())
        } else {
            finish(bytes, context, self.server, &self.hits)
        }
    }
}
#[mod_api::__subscriptions]
impl Hooks for Probe {
    fn register(&self, registrar: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
        registrar
            .payload_handlers()
            .registrar("1")
            .executes_on(if self.network {
                HandlerThread::Network
            } else {
                HandlerThread::Main
            })
            .play_bidirectional(
                PayloadId(CHANNEL.into()),
                Arc::new(Handler {
                    expected: self.expected.clone(),
                    hits: self.server.clone(),
                    server: true,
                    network: self.network,
                }),
                Some(Arc::new(Handler {
                    expected: self.expected.clone(),
                    hits: self.client.clone(),
                    server: false,
                    network: self.network,
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
        &["one_mib", "network", "nine_mib"],
    ) {
        return;
    }
    let (size, network) = match std::env::var("HORIZON_RAW_TRANSPORT_CHILD")
        .unwrap()
        .as_str()
    {
        "one_mib" => (1 << 20, false),
        "network" => (1 << 20, true),
        "nine_mib" => (9 << 20, false),
        case => panic!("unknown transport fixture case {case}"),
    };
    let expected = Arc::new(
        (0..size)
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
            network,
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
