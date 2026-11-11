//! Real guest callback: exact bytes, context metadata, and a retained work resource.
use mod_api::*;
use std::sync::Arc;

pub fn register(registrar: &mut dyn Registrar) {
    registrar
        .payload_handlers()
        .registrar("exact-v1")
        .executes_on(HandlerThread::Network)
        .play_bidirectional(
            PayloadId("wasm_fixture:raw".into()),
            Arc::new(Handler),
            Some(Arc::new(Handler)),
        );
}
struct Handler;
impl IPayloadHandler for Handler {
    fn handle(&self, bytes: PayloadBytes, cx: &mut dyn IPayloadContext) -> Result<(), String> {
        if bytes.0.len() == 1 {
            cx.enqueue_work(Arc::new(TrapWork)).unwrap();
            if bytes.0[0] == 1 {
                cx.reply(
                    &PayloadId("wasm_fixture:raw".into()),
                    PayloadBytes(vec![4, 5]),
                )
                .unwrap();
                panic!("handler trap after staging reply and work");
            }
            return Ok(());
        }
        assert_eq!(bytes.0.len(), 1024 * 1024);
        for (i, byte) in bytes.0.iter().enumerate() {
            assert_eq!(*byte, (i % 251) as u8);
        }
        assert_eq!(cx.protocol(), PayloadProtocol::Play);
        assert_eq!(cx.player().unwrap().uuid(), 99);
        assert!(matches!(
            cx.server_host(),
            Err(PayloadContextError::WrongThread)
        ));
        assert!(matches!(
            cx.client_host(),
            Err(PayloadContextError::WrongThread)
        ));
        assert_eq!(
            cx.reply(&PayloadId("wasm_fixture:raw".into()), bytes),
            Ok(PayloadAccepted::Queued)
        );
        assert_eq!(
            cx.enqueue_work(Arc::new(Work(cx.flow()))),
            Ok(PayloadWorkStatus::Queued)
        );
        Ok(())
    }
}
struct Work(PayloadDirection);
impl PayloadWork for Work {
    fn run(&self, cx: &mut dyn IPayloadContext) -> Result<(), String> {
        assert_eq!(cx.flow(), self.0);
        match self.0 {
            PayloadDirection::ToServer => {
                assert!(matches!(
                    cx.client_host(),
                    Err(PayloadContextError::WrongSide)
                ));
                let host = cx.server_host().unwrap();
                assert_eq!(
                    host.item_key(&Identifier::parse("minecraft:stone").unwrap()),
                    None
                );
            }
            PayloadDirection::ToClient => {
                assert!(matches!(
                    cx.server_host(),
                    Err(PayloadContextError::WrongSide)
                ));
                assert_eq!(
                    cx.client_host().unwrap().send_payload_to_server(
                        &PayloadId("wasm_fixture:raw".into()),
                        PayloadBytes(vec![9, 8, 7])
                    ),
                    Ok(PayloadAccepted::Queued)
                );
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}

struct TrapWork;
impl PayloadWork for TrapWork {
    fn run(&self, cx: &mut dyn IPayloadContext) -> Result<(), String> {
        cx.reply(
            &PayloadId("wasm_fixture:raw".into()),
            PayloadBytes(vec![6, 7]),
        )
        .unwrap();
        panic!("work trap after staging reply");
    }
}
