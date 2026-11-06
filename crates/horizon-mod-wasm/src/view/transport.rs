//! Stock canonical list cursors on input, and store-owned buffers retained until lowering ends.
use super::*;
use std::sync::Arc;
use wasmtime::component::{Access, ResourceType, WasmList};
use wasmtime::{AsContextMut, StoreContextMut};

type Get = Result<Arc<Vec<wire::Cell>>, wire::Refusal>;
type Read = (Resource<wire::Node>, u32);
type Write = (Resource<wire::Node>, u32, WasmList<wire::Cell>);

#[derive(Default)]
struct Buffers {
    cells: Vec<Arc<Vec<wire::Cell>>>,
    pending: Vec<internals::Value>,
    gets: Arc<Vec<Get>>,
    sets: Arc<Vec<Result<(), wire::Refusal>>>,
    inputs: Vec<Vec<wire::Cell>>,
    requests: Vec<Read>,
}

impl Buffers {
    fn begin(&mut self) {
        Arc::get_mut(&mut self.gets).unwrap().clear();
        Arc::get_mut(&mut self.sets).unwrap().clear();
        self.requests.clear();
    }

    fn encode(&mut self, index: usize, value: internals::Value) -> Get {
        crate::frame::value_size(&value)?;
        while self.cells.len() <= index {
            self.cells.push(Arc::new(Vec::new()));
        }
        let cells = Arc::get_mut(&mut self.cells[index]).unwrap();
        cells.clear();
        internals::flatten_into(value, cells, &mut self.pending);
        Ok(self.cells[index].clone())
    }

    fn input(
        &mut self,
        store: &mut StoreContextMut<'_, State>,
        cells: WasmList<wire::Cell>,
        index: usize,
    ) -> wasmtime::Result<usize> {
        while self.inputs.len() <= index {
            self.inputs.push(Vec::new());
        }
        let input = &mut self.inputs[index];
        input.clear();
        for cell in cells.iter(store.as_context_mut())? {
            input.push(cell?);
        }
        Ok(wire_cost(input))
    }
}

fn get_many(
    store: StoreContextMut<'_, State>,
    reads: WasmList<Read>,
) -> wasmtime::Result<Arc<Vec<Get>>> {
    State::with_buffer::<Buffers, _>(store, |buffers, mut store| {
        contain(|| {
            buffers.begin();
            let mut cost = reads.len().max(1);
            for read in reads.iter(store.as_context_mut())? {
                let (node, field) = read?;
                cost = cost
                    .saturating_add(node_cost(&node))
                    .saturating_add(journal_cost());
                buffers.requests.push((node, field));
            }
            charge(&mut store, cost)?;
            let mut output = 0usize;
            for index in 0..buffers.requests.len() {
                let (node, field) = &buffers.requests[index];
                let value = View
                    .get_value(node, *field)
                    .and_then(|v| buffers.encode(index, v));
                if let Ok(cells) = &value {
                    output = output.saturating_add(wire_cost(cells));
                }
                Arc::get_mut(&mut buffers.gets).unwrap().push(value);
            }
            charge(&mut store, output)?;
            Ok(buffers.gets.clone())
        })
    })
}

fn set_many(
    store: StoreContextMut<'_, State>,
    writes: WasmList<Write>,
) -> wasmtime::Result<Arc<Vec<Result<(), wire::Refusal>>>> {
    State::with_buffer::<Buffers, _>(store, |buffers, mut store| {
        contain(|| {
            buffers.begin();
            let mut cost = writes.len().max(1);
            for index in 0..writes.len() {
                let (node, field, cells) = writes.get(store.as_context_mut(), index).unwrap()?;
                cost = cost
                    .saturating_add(node_cost(&node))
                    .saturating_add(buffers.input(&mut store, cells, index)?);
                buffers.requests.push((node, field));
            }
            charge(&mut store, cost)?;
            for ((node, field), cells) in buffers.requests.drain(..).zip(&mut buffers.inputs) {
                Arc::get_mut(&mut buffers.sets)
                    .unwrap()
                    .push(View.set_buffer(&node, field, cells));
            }
            Ok(buffers.sets.clone())
        })
    })
}

fn get(
    store: StoreContextMut<'_, State>,
    node: Resource<wire::Node>,
    field: Option<u32>,
) -> wasmtime::Result<Get> {
    State::with_buffer::<Buffers, _>(store, |buffers, mut store| {
        contain(|| {
            buffers.begin();
            charge(
                &mut store,
                1usize
                    .saturating_add(node_cost(&node))
                    .saturating_add(journal_cost()),
            )?;
            let value = match field {
                Some(field) => View.get_value(&node, field),
                None => current(|a| a.frame.read(a.node(&node)?).map_err(Into::into)),
            }
            .and_then(|v| buffers.encode(0, v));
            charge(&mut store, value.as_ref().map_or(0, |v| wire_cost(v)))?;
            Ok(value)
        })
    })
}

fn set(
    store: StoreContextMut<'_, State>,
    node: Resource<wire::Node>,
    field: u32,
    cells: WasmList<wire::Cell>,
) -> wasmtime::Result<Result<(), wire::Refusal>> {
    State::with_buffer::<Buffers, _>(store, |buffers, mut store| {
        contain(|| {
            buffers.begin();
            let cost = 1usize
                .saturating_add(node_cost(&node))
                .saturating_add(buffers.input(&mut store, cells, 0)?);
            charge(&mut store, cost)?;
            Ok(View.set_buffer(&node, field, &mut buffers.inputs[0]))
        })
    })
}

pub(super) fn add_to_linker(linker: &mut Linker<State>) -> wasmtime::Result<()> {
    let mut interface = linker.instance("horizon:mod-internals/view@0.1.0")?;
    interface.resource("node", ResourceType::host::<wire::Node>(), |store, rep| {
        <View as wire::HostNodeWithStore<State>>::drop(
            Access::new(store, |_| View),
            Resource::new_own(rep),
        )
    })?;
    macro_rules! forward {
        ($trait:ident, $method:ident, $name:literal, $($arg:ident: $ty:ty),*) => {
            interface.func_wrap($name, |store: StoreContextMut<'_, State>, ($($arg,)*): ($($ty,)*)| {
                Ok((<View as wire::$trait<State>>::$method(Access::new(store, |_| View), $($arg),*)?,))
            })?;
        };
    }
    forward!(HostNodeWithStore, field, "[method]node.field", node: Resource<wire::Node>, n: u32);
    forward!(HostNodeWithStore, index, "[method]node.index", node: Resource<wire::Node>, i: u64);
    forward!(HostNodeWithStore, key, "[method]node.key", node: Resource<wire::Node>, k: Vec<wire::Cell>);
    forward!(HostNodeWithStore, len, "[method]node.len", node: Resource<wire::Node>);
    forward!(HostNodeWithStore, case, "[method]node.case", node: Resource<wire::Node>);
    forward!(HostWithStore, root, "root", n: u32);
    forward!(HostWithStore, call, "call", declared: u32, receiver: Option<Resource<wire::Node>>, args: Vec<wire::Arg>);
    interface.func_wrap("[method]node.get", |store, (node, field)| {
        Ok((get(store, node, Some(field))?,))
    })?;
    interface.func_wrap("[method]node.read", |store, (node,)| {
        Ok((get(store, node, None)?,))
    })?;
    interface.func_wrap("[method]node.set", |store, (node, field, cells)| {
        Ok((set(store, node, field, cells)?,))
    })?;
    interface.func_wrap("get-many", |store, (reads,)| Ok((get_many(store, reads)?,)))?;
    interface.func_wrap("set-many", |store, (writes,)| {
        Ok((set_many(store, writes)?,))
    })?;
    Ok(())
}
