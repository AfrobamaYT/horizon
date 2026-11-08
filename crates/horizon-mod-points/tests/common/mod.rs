//! The loopback shared by the policy-fixture tests: one synchronous guest call per firing,
//! under a registration policy, with the encoded value's lifetime checked after apply.
use mod_api::wire::{Id, Reader, Record, ResourceCodec, ResourceScope, WireError, Writer};
use mod_api::{RowError, RowPages, RowWindow};
use mod_points::{__private::RawForward, endpoint_policy::EndpointPolicy, exposure::Plan};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Default)]
struct Resources(RefCell<Vec<Arc<dyn RowPages>>>);
impl ResourceCodec for Resources {
    fn store(&self, value: Box<dyn Any>) -> Result<Id, WireError> {
        let value = value
            .downcast::<Arc<dyn RowPages>>()
            .map_err(|_| WireError::Invalid("wrong resource"))?;
        let mut values = self.0.borrow_mut();
        values.push(*value);
        Ok(Id(values.len() as u64))
    }
    fn load(&self, id: Id, view: TypeId) -> Result<Box<dyn Any>, WireError> {
        if view != TypeId::of::<Arc<dyn RowPages>>() {
            return Err(WireError::Invalid("wrong view"));
        }
        self.0
            .borrow()
            .get(id.0 as usize - 1)
            .cloned()
            .map(|value| Box::new(value) as Box<dyn Any>)
            .ok_or(WireError::Invalid("unknown resource"))
    }
}

/// One listener's endpoint. `refused` is set by a test whose listener's edits must all be
/// refused: the apply then fails as a whole and the native call proceeds unchanged.
pub struct Endpoint {
    pub listener: u32,
    pub policy: Arc<EndpointPolicy>,
    pub refused: &'static AtomicBool,
    pub retained: Mutex<Option<Arc<dyn RowPages>>>,
}
impl RawForward for Endpoint {
    fn plan(&self, head_call: usize) -> Option<Arc<Plan>> {
        self.policy.plan(head_call)
    }
    fn forward(&self, id: u32, record: &mut dyn Record) {
        let resources = Rc::new(Resources::default());
        // SAFETY: this is the synchronous native guest loopback's actual Hook call frame.
        let _call = unsafe { ResourceScope::native_root(resources.clone()) };
        let mut input = Writer::new();
        input.put(&self.listener);
        record.encode(&mut input);
        assert_eq!(input.error(), None);
        let mut input = Reader::new(input.bytes());
        let mut reply = Writer::new();
        mod_points::guest_dispatch(id, &mut input, &mut reply).unwrap();
        let mut reply = Reader::new(reply.bytes());
        reply.take::<()>().unwrap();
        let applied = record.apply(reply.rest());
        assert_eq!(applied.is_err(), self.refused.load(Ordering::Relaxed));
        let source = resources.0.borrow().first().cloned().unwrap();
        // The call marker is still live: apply must already have closed the encoded value.
        assert_eq!(
            source.window(RowWindow {
                node: 0,
                start: 0,
                rows: 1,
                depth: 0
            }),
            Err(RowError::Stale)
        );
        *self.retained.lock().unwrap() = Some(source);
    }
}
