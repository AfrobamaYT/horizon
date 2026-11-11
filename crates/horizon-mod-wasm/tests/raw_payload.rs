//! Executes registered handlers in a compiled shared-memory wasm guest.
use mod_api::*;
use std::sync::Arc;
struct Native {
    reads: std::cell::Cell<usize>,
    writes: usize,
}
impl Host for Native {}
mod_api::impl_host_areas!(Native, except(items));
impl mod_api::prelude::ItemsHost for Native {
    fn item_key(&self, id: &Identifier) -> Option<ItemKey> {
        assert_eq!(id.to_string(), "minecraft:stone");
        self.reads.set(self.reads.get() + 1);
        None
    }
}
impl ServerHost for Native {}
mod_api::impl_server_host_areas!(Native);
impl ClientHost for Native {}
mod_api::impl_client_host_areas!(Native, except(payloads));
impl mod_api::prelude::PayloadsClientHost for Native {
    fn send_payload_to_server(
        &mut self,
        id: &PayloadId,
        bytes: PayloadBytes,
    ) -> Result<PayloadAccepted, PayloadSendError> {
        assert_eq!(id.0, "wasm_fixture:raw");
        assert_eq!(bytes.0, [9, 8, 7]);
        self.writes += 1;
        Ok(PayloadAccepted::Queued)
    }
}
struct Context {
    native: Native,
    flow: PayloadDirection,
    network: bool,
    replies: Vec<PayloadBytes>,
    work: Option<Arc<dyn PayloadWork>>,
}
impl IPayloadContext for Context {
    fn player(&self) -> Result<PlayerRef, PayloadContextError> {
        Ok(mod_api::__private::game::player_ref(
            EntityRef::new(WorldEpoch::new(7), 42),
            99,
        ))
    }
    fn flow(&self) -> PayloadDirection {
        self.flow
    }
    fn protocol(&self) -> PayloadProtocol {
        PayloadProtocol::Play
    }
    fn server_host(&mut self) -> Result<&mut dyn ServerHost, PayloadContextError> {
        if self.network {
            return Err(PayloadContextError::WrongThread);
        }
        if self.flow != PayloadDirection::ToServer {
            return Err(PayloadContextError::WrongSide);
        }
        Ok(&mut self.native)
    }
    fn client_host(&mut self) -> Result<&mut dyn ClientHost, PayloadContextError> {
        if self.network {
            return Err(PayloadContextError::WrongThread);
        }
        if self.flow != PayloadDirection::ToClient {
            return Err(PayloadContextError::WrongSide);
        }
        Ok(&mut self.native)
    }
    fn reply(
        &mut self,
        id: &PayloadId,
        bytes: PayloadBytes,
    ) -> Result<PayloadAccepted, PayloadSendError> {
        assert_eq!(id.0, "wasm_fixture:raw");
        self.replies.push(bytes);
        Ok(PayloadAccepted::Queued)
    }
    fn disconnect(&mut self, _: Text) {
        panic!("unexpected disconnect")
    }
    fn enqueue_work(
        &mut self,
        work: Arc<dyn PayloadWork>,
    ) -> Result<PayloadWorkStatus, PayloadContextError> {
        assert!(self.work.replace(work).is_none());
        Ok(PayloadWorkStatus::Queued)
    }
}
#[test]
#[ignore = "requires compiled HORIZON_WASM_FIXTURE and HORIZON_WASM_STD"]
fn raw_payload_callback() {
    use horizon_mod_package::manifest::Package;
    let path = std::env::var("HORIZON_WASM_FIXTURE").unwrap();
    let path = std::path::Path::new(&path);
    let manifest = Package::read(path).unwrap().manifest;
    let info = PackageInfo::new(
        manifest.id.clone(),
        manifest.version.clone(),
        PackageKind::Code,
        Side::Both,
        vec![manifest.id.clone()],
        vec![],
    );
    let loaded = horizon_mod_wasm::load(
        &std::fs::read(std::env::var("HORIZON_WASM_STD").unwrap()).unwrap(),
        vec![horizon_mod_wasm::WasmPackage {
            manifest,
            module: Package::read_wasm(path).unwrap(),
        }],
    );
    assert!(loaded.refused.is_empty(), "{:?}", loaded.refused);
    loaded.initialize(&[info]).unwrap();
    mod_api::testing::with_mods(loaded.mods.into(), || {
        mod_api::__private::install_parts(
            mod_api::__private::mods(),
            &mod_api::__private::part_tests::rule_schema(),
            |_| false,
        )
        .unwrap();
        mod_api::__private::mark_game_thread();
        let table = mod_api::__private::game::installed_payloads().unwrap();
        let declaration = table
            .declarations()
            .find(|d| d.id.0 == "wasm_fixture:raw")
            .unwrap();
        assert_eq!(declaration.version, "exact-v1");
        assert_eq!(declaration.thread, HandlerThread::Network);
        let body = PayloadBytes((0..1024 * 1024).map(|i| (i % 251) as u8).collect());
        for flow in [PayloadDirection::ToServer, PayloadDirection::ToClient] {
            let receiver = table
                .handler(&declaration.id.0, PayloadProtocol::Play, flow)
                .unwrap();
            let mut cx = Context {
                native: Native {
                    reads: 0.into(),
                    writes: 0,
                },
                flow,
                network: true,
                replies: vec![],
                work: None,
            };
            receiver.handle(body.clone(), &mut cx).unwrap();
            assert_eq!(cx.replies.as_slice(), std::slice::from_ref(&body));
            assert_eq!(cx.native.reads.get(), 0);
            assert_eq!(cx.native.writes, 0);
            let work = cx.work.take().expect("guest work survives callback return");
            cx.network = false;
            receiver.run_work(&*work, &mut cx).unwrap();
            assert_eq!(
                cx.native.reads.get(),
                usize::from(flow == PayloadDirection::ToServer)
            );
            assert_eq!(
                cx.native.writes,
                usize::from(flow == PayloadDirection::ToClient)
            );
        }
    });
}
