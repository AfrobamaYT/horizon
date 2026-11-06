//! Stock canonical list cursors on input, and store-owned buffers retained until lowering ends.
use super::lazy::Materialize;
use super::*;
use std::sync::Arc;
use wasmtime::component::WasmList;
use wasmtime::{AsContextMut, StoreContextMut};

type Get = Result<u32, wire::Refusal>;
type Read = (Resource<wire::Node>, u32);
type Write = (Resource<wire::Node>, u32, u32);

#[derive(wasmtime::component::ComponentType, wasmtime::component::Lower)]
#[component(record)]
struct BatchValues {
    entries: Arc<Vec<Get>>,
    cells: Arc<Vec<wire::Cell>>,
}

#[derive(Default)]
pub(super) struct Buffers {
    cells: Arc<Vec<wire::Cell>>,
    pending: Vec<internals::Value>,
    gets: Arc<Vec<Get>>,
    sets: Arc<Vec<Result<(), wire::Refusal>>>,
    pub(super) inputs: Vec<Vec<wire::Cell>>,
    requests: Vec<Read>,
    refused: Vec<bool>,
}

impl Buffers {
    fn begin(&mut self) {
        Arc::get_mut(&mut self.gets).unwrap().clear();
        Arc::get_mut(&mut self.sets).unwrap().clear();
        Arc::get_mut(&mut self.cells).unwrap().clear();
        self.requests.clear();
        self.refused.clear();
    }

    pub(super) fn clear_inputs(&mut self) {
        for input in &mut self.inputs {
            input.clear();
            input.shrink_to(1024);
        }
    }

    fn encode(&mut self, value: internals::Value) -> Get {
        crate::frame::value_size(&value)?;
        let cells = Arc::get_mut(&mut self.cells).unwrap();
        let start = cells.len();
        internals::flatten_into(value, cells, &mut self.pending);
        Ok((cells.len() - start) as u32)
    }

    fn input(
        &mut self,
        store: &mut StoreContextMut<'_, State>,
        cells: &WasmList<lazy::Cell>,
        index: usize,
        start: usize,
        len: usize,
    ) -> wasmtime::Result<bool> {
        while self.inputs.len() <= index {
            self.inputs.push(Vec::new());
        }
        if len > 65_536 {
            return Ok(false);
        }
        let input = &mut self.inputs[index];
        input.clear();
        let mut source = lazy::Input::new(store);
        for offset in start..start + len {
            let cell = cells.get(&mut *source.store, offset).unwrap()?;
            let Some(cell) = cell.materialize(&mut source)? else {
                return Ok(false);
            };
            input.push(cell);
        }
        Ok(true)
    }
}

fn get_many(
    store: StoreContextMut<'_, State>,
    reads: WasmList<Read>,
) -> wasmtime::Result<(Arc<Vec<Get>>, Arc<Vec<wire::Cell>>)> {
    bounded_batch(reads.len())?;
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
                let value = View.get_value(node, *field).and_then(|v| buffers.encode(v));
                if let Ok(len) = value {
                    let cells = &buffers.cells[buffers.cells.len() - len as usize..];
                    output = output.saturating_add(wire_cost(cells));
                }
                Arc::get_mut(&mut buffers.gets).unwrap().push(value);
            }
            charge(&mut store, output)?;
            Ok((buffers.gets.clone(), buffers.cells.clone()))
        })
    })
}

fn set_many(
    store: StoreContextMut<'_, State>,
    writes: WasmList<Write>,
    cells: WasmList<lazy::Cell>,
) -> wasmtime::Result<Arc<Vec<Result<(), wire::Refusal>>>> {
    bounded_batch(writes.len())?;
    State::with_buffer::<Buffers, _>(store, |buffers, mut store| {
        let result = contain(|| {
            #[cfg(test)]
            let fuel_before = store.get_fuel().ok();
            buffers.begin();
            charge(&mut store, writes.len().max(1))?;
            let mut start = 0usize;
            for index in 0..writes.len() {
                let (node, field, len) = writes.get(store.as_context_mut(), index).unwrap()?;
                let len = len as usize;
                let end = start
                    .checked_add(len)
                    .filter(|end| *end <= cells.len())
                    .ok_or_else(|| wasmtime::Error::msg("invalid internals batch lengths"))?;
                charge(&mut store, node_cost(&node).saturating_add(len))?;
                let accepted = buffers.input(&mut store, &cells, index, start, len)?;
                start = end;
                buffers.refused.push(!accepted);
                buffers.requests.push((node, field));
            }
            wasmtime::ensure!(start == cells.len(), "invalid internals batch lengths");
            for (index, (node, field)) in buffers.requests.drain(..).enumerate() {
                let value = if buffers.refused[index] {
                    Err(wire::Refusal::NotHere("internals budget".into()))
                } else {
                    View.set_buffer(&node, field, &mut buffers.inputs[index])
                };
                Arc::get_mut(&mut buffers.sets).unwrap().push(value);
            }
            #[cfg(test)]
            if let Some(before) = fuel_before {
                tests::FUEL_DEBITS
                    .with_borrow_mut(|debits| debits.push(before - store.get_fuel().unwrap()));
            }
            Ok(buffers.sets.clone())
        });
        buffers.clear_inputs();
        result
    })
}

pub(super) fn add_to_linker(linker: &mut Linker<State>) -> wasmtime::Result<()> {
    linker.allow_shadowing(true);
    let mut interface = linker.instance("horizon:mod-internals/view@0.1.0")?;
    interface.func_wrap("get-many", |store, (reads,)| {
        let (entries, cells) = get_many(store, reads)?;
        Ok((BatchValues { entries, cells },))
    })?;
    interface.func_wrap("set-many", |store, (writes, cells)| {
        Ok((set_many(store, writes, cells)?,))
    })?;
    linker.allow_shadowing(false);
    Ok(())
}
