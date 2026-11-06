//! The generic internals imports over one raw call's frame. Like the mod API's call frame,
//! pointers and nodes stay on the calling thread, never in the group's Send store data.
use crate::frame::{Call, Frame, Node, Reached};
use mod_api::__private::wasm_host::State;
use mod_points::internals::bindings::horizon::mod_internals::view as wire;
use mod_points::internals::{self, Table, flatten, unflatten};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use wasmtime::component::{HasData, Linker, Resource};

/// A declared-list entry resolved once at load.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolved {
    Type(u32),
    Thunk(u32),
    Place(usize),
    Member { owner: u32, field: u32 },
}

/// Resolves only fingerprinted items; wrappers are reached from checked named types.
pub fn resolve(
    table: &Table,
    mod_id: &str,
    declared: &[(&str, &str)],
) -> Result<Vec<Resolved>, crate::Refusal> {
    if declared.is_empty() {
        return Ok(Vec::new());
    }
    if let Some(&(path, _)) = declared
        .iter()
        .find(|(path, _)| table.native_only.contains(path))
    {
        return Err(crate::Refusal::missing_internal(table, mod_id, path));
    }
    if table.types.is_empty()
        && table.thunks.is_empty()
        && table.places.is_empty()
        && table.members.is_empty()
        && table.native_only.is_empty()
    {
        return Err(crate::Refusal::NoInternalsTable);
    }
    declared
        .iter()
        .map(|&(path, hash)| {
            let candidates = table
                .types
                .iter()
                .enumerate()
                .filter(|(_, ty)| ty.path == path && !ty.hash.is_empty())
                .map(|(i, ty)| (Resolved::Type(i as u32), ty.hash))
                .chain(
                    table
                        .thunks
                        .iter()
                        .enumerate()
                        .filter(|(_, thunk)| thunk.path == path && !thunk.hash.is_empty())
                        .map(|(i, thunk)| (Resolved::Thunk(i as u32), thunk.hash)),
                )
                .chain(
                    table
                        .members
                        .iter()
                        .filter(|member| member.path == path && !member.hash.is_empty())
                        .map(|member| {
                            (
                                Resolved::Member {
                                    owner: member.owner,
                                    field: member.field,
                                },
                                member.hash,
                            )
                        }),
                )
                .chain(
                    table
                        .places
                        .iter()
                        .enumerate()
                        .filter(|(_, place)| place.target == path && !place.hash.is_empty())
                        .map(|(i, place)| (Resolved::Place(i), place.hash)),
                );
            let mut present = false;
            for (resolved, found) in candidates {
                present = true;
                if found != hash {
                    continue;
                }
                if let Resolved::Member { owner, .. } = resolved {
                    let owner = table
                        .types
                        .get(owner as usize)
                        .ok_or_else(|| crate::Refusal::ChangedInternal { path: path.into() })?;
                    if !declared.contains(&(owner.path, owner.hash)) {
                        return Err(crate::Refusal::ChangedInternal {
                            path: owner.path.into(),
                        });
                    }
                }
                return Ok(resolved);
            }
            Err(if present {
                crate::Refusal::ChangedInternal { path: path.into() }
            } else {
                crate::Refusal::missing_internal(table, mod_id, path)
            })
        })
        .collect()
}

struct Active<'a> {
    frame: Frame<'a>,
    nodes: BTreeMap<u32, Node>,
    declared: &'a [Resolved],
}

thread_local! {
    static CURRENT: Cell<*mut Active<'static>> = const { Cell::new(std::ptr::null_mut()) };
}
// Never reuse a resource rep, even if a group's next call runs on a different thread.
static NEXT_NODE: AtomicU32 = AtomicU32::new(1);

fn gone(why: &str) -> wire::Refusal {
    wire::Refusal::Gone(why.into())
}

fn current<T>(
    f: impl FnOnce(&mut Active<'_>) -> Result<T, wire::Refusal>,
) -> Result<T, wire::Refusal> {
    // SAFETY: `run` installs a stack frame for the synchronous call, restoring it before it
    // drops. Imports run serially on that thread; no reference escapes this closure.
    let active = unsafe { CURRENT.get().as_mut() }.ok_or_else(|| gone("no call is running"))?;
    f(active)
}

impl Active<'_> {
    fn node(&self, resource: &Resource<wire::Node>) -> Result<&Node, wire::Refusal> {
        self.nodes
            .get(&resource.rep())
            .ok_or_else(|| gone("the node's call is over"))
    }

    fn keep(&mut self, node: Node) -> Result<Resource<wire::Node>, wire::Refusal> {
        let rep = NEXT_NODE
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| gone("node resource ids exhausted"))?;
        self.nodes.insert(rep, node);
        Ok(Resource::new_own(rep))
    }

    fn reached(&mut self, reached: Reached) -> Result<wire::Reached, wire::Refusal> {
        match reached {
            Reached::Node(node) => self.keep(node).map(wire::Reached::Node),
            Reached::Value(value) => Ok(wire::Reached::Value(flatten(value))),
        }
    }
}

struct Scope(*mut Active<'static>);
impl Drop for Scope {
    fn drop(&mut self) {
        CURRENT.set(self.0);
    }
}

/// Runs a raw listener with a call-scoped view; commits only when the listener returns normally.
/// A trap drops its pending writes, and the scope guard restores the previous thread frame.
pub fn run<T>(
    mut frame: Frame<'_>,
    declared: &[Resolved],
    f: impl FnOnce() -> wasmtime::Result<T>,
) -> wasmtime::Result<T> {
    frame.allow_members(declared.iter().filter_map(|entry| match entry {
        Resolved::Member { owner, field } => Some((*owner, *field)),
        _ => None,
    }));
    let mut active = Active {
        frame,
        nodes: BTreeMap::new(),
        declared,
    };
    let scope = Scope(CURRENT.replace((&raw mut active).cast()));
    let result = f();
    drop(scope);
    if result.is_ok() {
        log_refused(active.frame.apply());
    }
    result
}

fn log_refused(
    refused: Vec<(
        crate::journal::Path<internals::Value>,
        u32,
        internals::Refusal,
    )>,
) {
    for (path, field, why) in refused {
        log::warn!("wasm internals: dropped write at {path:?} field {field}: {why:?}");
    }
}

/// A stateless host adapter: all imports use the innermost call on the current thread.
pub struct View;
impl HasData for View {
    type Data<'a> = View;
}

/// Adds the internals imports to a group's linker.
pub fn add_to_linker(linker: &mut Linker<State>) {
    wire::add_to_linker::<State, View>(linker, |_| View)
        .expect("the game's internals imports are defined once");
}

impl wire::HostNode for View {
    fn field(
        &mut self,
        node: Resource<wire::Node>,
        n: u32,
    ) -> Result<Resource<wire::Node>, wire::Refusal> {
        current(|a| {
            let next = a.frame.field(a.node(&node)?, n)?;
            a.keep(next)
        })
    }
    fn get(
        &mut self,
        node: Resource<wire::Node>,
        n: u32,
    ) -> Result<Vec<wire::Cell>, wire::Refusal> {
        current(|a| Ok(flatten(a.frame.get(a.node(&node)?, n)?)))
    }
    fn set(
        &mut self,
        node: Resource<wire::Node>,
        n: u32,
        v: Vec<wire::Cell>,
    ) -> Result<(), wire::Refusal> {
        current(|a| {
            let node = a
                .nodes
                .get(&node.rep())
                .ok_or_else(|| gone("the node's call is over"))?;
            a.frame.set(node, n, unflatten(v)?).map_err(Into::into)
        })
    }
    fn index(
        &mut self,
        node: Resource<wire::Node>,
        i: u64,
    ) -> Result<wire::Reached, wire::Refusal> {
        current(|a| {
            let next = a.frame.index(a.node(&node)?, i)?;
            a.reached(next)
        })
    }
    fn key(
        &mut self,
        node: Resource<wire::Node>,
        k: Vec<wire::Cell>,
    ) -> Result<wire::Reached, wire::Refusal> {
        current(|a| {
            let next = a.frame.key(a.node(&node)?, unflatten(k)?)?;
            a.reached(next)
        })
    }
    fn len(&mut self, node: Resource<wire::Node>) -> Result<u64, wire::Refusal> {
        current(|a| a.frame.len(a.node(&node)?).map_err(Into::into))
    }
    fn case(&mut self, node: Resource<wire::Node>) -> Result<u32, wire::Refusal> {
        current(|a| a.frame.case(a.node(&node)?).map_err(Into::into))
    }
    fn read(&mut self, node: Resource<wire::Node>) -> Result<Vec<wire::Cell>, wire::Refusal> {
        current(|a| Ok(flatten(a.frame.read(a.node(&node)?)?)))
    }
    fn drop(&mut self, node: Resource<wire::Node>) -> wasmtime::Result<()> {
        let _ = current(|a| {
            a.nodes.remove(&node.rep());
            Ok(())
        });
        Ok(())
    }
}

impl wire::Host for View {
    fn root(&mut self, n: u32) -> Result<Resource<wire::Node>, wire::Refusal> {
        current(|a| {
            let node = a.frame.root(n)?;
            a.keep(node)
        })
    }
    fn call(
        &mut self,
        declared: u32,
        receiver: Option<Resource<wire::Node>>,
        args: Vec<wire::Arg>,
    ) -> Result<wire::Reached, wire::Refusal> {
        current(|a| {
            let Some(Resolved::Thunk(function)) = a.declared.get(declared as usize) else {
                return Err(wire::Refusal::NotHere("not a declared function".into()));
            };
            if let Some(receiver) = &receiver {
                a.nodes
                    .get_mut(&receiver.rep())
                    .ok_or_else(|| gone("the node's call is over"))?
                    .restricted = true;
            }
            let nodes = &a.nodes;
            let node = |r: &Resource<wire::Node>| {
                nodes
                    .get(&r.rep())
                    .ok_or_else(|| gone("the node's call is over"))
            };
            let receiver = receiver.as_ref().map(node).transpose()?;
            let args = args
                .into_iter()
                .map(|arg| match arg {
                    wire::Arg::Value(v) => Ok(Call::Value(unflatten(v)?)),
                    wire::Arg::Node(r) => node(&r).map(Call::Node),
                })
                .collect::<Result<Vec<_>, wire::Refusal>>()?;
            // A game function may dispatch into another wasm group. Its imports must not
            // see this listener's frame, nor alias the mutable borrow held by this call.
            let mask = Scope(CURRENT.replace(std::ptr::null_mut()));
            let (reached, refused) = a.frame.call(*function, receiver, args)?;
            drop(mask);
            log_refused(refused);
            a.reached(reached)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::tests::{TABLE, frame, world};
    use wire::{Host, HostNode};

    fn borrow(node: &Resource<wire::Node>) -> Resource<wire::Node> {
        Resource::new_borrow(node.rep())
    }

    #[test]
    fn a_binding_reads_writes_and_calls_the_table() {
        let mut game = world();
        run(
            frame(&mut game),
            &[
                Resolved::Thunk(0),
                Resolved::Member { owner: 0, field: 0 },
                Resolved::Member { owner: 0, field: 1 },
            ],
            || {
                let mut host = View;
                let root = host.root(0).unwrap();
                assert_eq!(
                    unflatten(host.get(borrow(&root), 1).unwrap()).unwrap(),
                    internals::Value::B(false)
                );
                host.set(borrow(&root), 1, vec![wire::Cell::B(true)])
                    .unwrap();
                assert_eq!(
                    unflatten(host.get(borrow(&root), 1).unwrap()).unwrap(),
                    internals::Value::B(true)
                );
                let players = host.field(borrow(&root), 0).unwrap();
                assert_eq!(host.len(borrow(&players)).unwrap(), 3);
                let wire::Reached::Node(first) = host.index(borrow(&players), 0).unwrap() else {
                    panic!("a node")
                };
                assert_eq!(
                    unflatten(host.read(borrow(&first)).unwrap()).unwrap(),
                    internals::Value::Rec(vec![internals::Value::S(10)])
                );
                let wire::Reached::Value(left) = host.call(0, Some(borrow(&root)), vec![]).unwrap()
                else {
                    panic!("a value")
                };
                assert_eq!(unflatten(left).unwrap(), internals::Value::U(2));
                assert_eq!(host.len(borrow(&players)).unwrap(), 2);
                assert!(matches!(
                    host.call(1, Some(borrow(&root)), vec![]),
                    Err(wire::Refusal::NotHere(_))
                ));
                Ok(())
            },
        )
        .unwrap();
        // Successful return applied the flag write before the thunk.
        let next = frame(&mut game);
        assert_eq!(
            next.get(&next.root(0).unwrap(), 1).unwrap(),
            internals::Value::B(true)
        );
    }

    fn assert_undeclared_field<T: std::fmt::Debug>(result: Result<T, wire::Refusal>) {
        match result {
            Err(wire::Refusal::NotHere(why)) => assert_eq!(why, "undeclared field"),
            other => panic!("expected an undeclared field refusal, got {other:?}"),
        }
    }

    #[test]
    fn receiver_fields_are_permissions_not_the_union_of_the_tables_members() {
        let mut game = world();
        let own = resolve(
            &TABLE,
            "own",
            &[("Game", "game-hash"), ("Game::kick", "field-kick-hash")],
        )
        .unwrap();
        let other = resolve(
            &TABLE,
            "other",
            &[("Game", "game-hash"), ("Game::flag", "flag-hash")],
        )
        .unwrap();
        run(frame(&mut game), &[], || {
            let root = View.root(0).unwrap();
            assert_undeclared_field(View.field(borrow(&root), 0));
            assert_undeclared_field(View.get(borrow(&root), 1));
            assert_undeclared_field(View.set(borrow(&root), 1, vec![wire::Cell::B(true)]));
            Ok(())
        })
        .unwrap();
        run(frame(&mut game), &own, || {
            let root = View.root(0).unwrap();
            let players = View.field(borrow(&root), 0).unwrap();
            let wire::Reached::Node(first) = View.index(borrow(&players), 0).unwrap() else {
                panic!("a player")
            };
            assert_eq!(
                unflatten(View.get(borrow(&first), 0).unwrap()).unwrap(),
                internals::Value::S(10)
            );
            View.set(borrow(&first), 0, vec![wire::Cell::S(17)])
                .unwrap();
            assert_eq!(
                unflatten(View.get(borrow(&first), 0).unwrap()).unwrap(),
                internals::Value::S(17)
            );
            assert_undeclared_field(View.get(borrow(&root), 1));
            assert_undeclared_field(View.set(borrow(&root), 1, vec![wire::Cell::B(true)]));
            Ok(())
        })
        .unwrap();
        run(frame(&mut game), &other, || {
            let root = View.root(0).unwrap();
            assert_eq!(
                unflatten(View.get(borrow(&root), 1).unwrap()).unwrap(),
                internals::Value::B(false)
            );
            View.set(borrow(&root), 1, vec![wire::Cell::B(true)])
                .unwrap();
            assert_undeclared_field(View.field(borrow(&root), 0));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn raw_record_fields_are_open_even_when_the_record_is_root_zero() {
        for receiver in [false, true] {
            let mut game = world();
            let root = crate::frame::Root {
                ptr: (&raw mut game).cast(),
                ty: 0,
                writable: true,
                receiver,
                born: None,
            };
            // SAFETY: this synthetic record has the fixture Game's layout and exclusive borrow.
            let frame = unsafe { Frame::new(&TABLE, vec![root]) };
            run(frame, &[], || {
                let root = View.root(0).unwrap();
                if receiver {
                    assert_undeclared_field(View.field(borrow(&root), 0));
                } else {
                    let players = View.field(borrow(&root), 0).unwrap();
                    assert_eq!(View.len(borrow(&players)).unwrap(), 3);
                    assert_eq!(
                        unflatten(View.get(borrow(&root), 1).unwrap()).unwrap(),
                        internals::Value::B(false)
                    );
                    View.set(borrow(&root), 1, vec![wire::Cell::B(true)])
                        .unwrap();
                    assert_eq!(
                        unflatten(View.get(borrow(&root), 1).unwrap()).unwrap(),
                        internals::Value::B(true)
                    );
                }
                Ok(())
            })
            .unwrap();
        }
    }

    #[test]
    fn a_subtree_node_used_as_a_call_receiver_becomes_restricted() {
        let mut game = world();
        run(
            frame(&mut game),
            &[Resolved::Member { owner: 0, field: 0 }, Resolved::Thunk(3)],
            || {
                let root = View.root(0).unwrap();
                let players = View.field(borrow(&root), 0).unwrap();
                let wire::Reached::Node(first) = View.index(borrow(&players), 0).unwrap() else {
                    panic!("a player")
                };
                assert_eq!(
                    unflatten(View.read(borrow(&first)).unwrap()).unwrap(),
                    internals::Value::Rec(vec![internals::Value::S(10)])
                );
                View.call(1, Some(borrow(&first)), vec![]).unwrap();
                assert_undeclared_field(View.get(borrow(&first), 0));
                assert_undeclared_field(View.set(borrow(&first), 0, vec![wire::Cell::S(0)]));
                assert_undeclared_field(View.read(borrow(&first)));
                Ok(())
            },
        )
        .unwrap();
    }

    #[test]
    fn a_returned_reference_has_an_unrestricted_subtree() {
        let mut game = world();
        run(frame(&mut game), &[Resolved::Thunk(1)], || {
            let root = View.root(0).unwrap();
            let wire::Reached::Node(first) = View.call(0, Some(borrow(&root)), vec![]).unwrap()
            else {
                panic!("a player")
            };
            assert_eq!(
                unflatten(View.get(borrow(&first), 0).unwrap()).unwrap(),
                internals::Value::S(10)
            );
            View.set(borrow(&first), 0, vec![wire::Cell::S(17)])
                .unwrap();
            assert_eq!(
                unflatten(View.get(borrow(&first), 0).unwrap()).unwrap(),
                internals::Value::S(17)
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn no_frame_and_a_node_kept_into_the_next_call_are_gone() {
        let mut host = View;
        assert!(
            matches!(host.root(0), Err(wire::Refusal::Gone(why)) if why == "no call is running")
        );
        assert!(matches!(
            host.set(Resource::new_borrow(0), 0, vec![]),
            Err(wire::Refusal::Gone(_))
        ));
        let mut game = world();
        let old = run(frame(&mut game), &[], || Ok(host.root(0).unwrap())).unwrap();
        assert!(matches!(
            host.get(borrow(&old), 1),
            Err(wire::Refusal::Gone(_))
        ));
        run(frame(&mut game), &[], || {
            let new = host.root(0).unwrap();
            assert_ne!(old.rep(), new.rep());
            assert!(matches!(
                host.get(borrow(&old), 1),
                Err(wire::Refusal::Gone(_))
            ));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn a_trap_discards_writes_and_clears_the_scope() {
        let mut game = world();
        let result: wasmtime::Result<()> = run(
            frame(&mut game),
            &[Resolved::Member { owner: 0, field: 1 }],
            || {
                let root = View.root(0).unwrap();
                View.set(borrow(&root), 1, vec![wire::Cell::B(true)])
                    .unwrap();
                Err(wasmtime::Error::msg("guest trapped"))
            },
        );
        assert!(result.is_err());
        assert!(matches!(View.root(0), Err(wire::Refusal::Gone(_))));
        let next = frame(&mut game);
        assert_eq!(
            next.get(&next.root(0).unwrap(), 1).unwrap(),
            internals::Value::B(false)
        );
    }

    #[test]
    fn a_thunk_cannot_reenter_its_listeners_view() {
        unsafe fn reenter(
            _: Option<*mut ()>,
            _: Vec<internals::Arg>,
        ) -> Result<internals::Reply, internals::Refusal> {
            assert!(matches!(View.root(0), Err(wire::Refusal::Gone(_))));
            Ok(internals::Reply::Value(internals::Value::None))
        }
        static TABLE: Table = Table {
            types: &[],
            thunks: &[internals::Thunk {
                path: "reenter",
                hash: "hash",
                receiver: None,
                args: &[],
                writes: 0,
                call: reenter,
            }],
            places: &[],
            members: &[],
            native_only: &[],
        };
        // SAFETY: the test has no game roots; its thunk only checks the import's refusal.
        let frame = unsafe { Frame::new(&TABLE, vec![]) };
        run(frame, &[Resolved::Thunk(0)], || {
            assert!(matches!(
                View.call(0, None, vec![]),
                Ok(wire::Reached::Value(_))
            ));
            // The mask restored the active frame, rather than leaving the imports outside it.
            assert!(matches!(
                View.call(1, None, vec![]),
                Err(wire::Refusal::NotHere(_))
            ));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn a_declared_field_checks_its_owner_and_disambiguates_a_method_path() {
        assert_eq!(
            resolve(&TABLE, "test", &[("Game::flag", "flag-hash")]),
            Err(crate::Refusal::ChangedInternal {
                path: "Game".into()
            })
        );
        assert_eq!(
            resolve(
                &TABLE,
                "test",
                &[
                    ("Game", "game-hash"),
                    ("Game::flag", "flag-hash"),
                    ("Game::kick", "field-kick-hash"),
                    ("Game::kick", "kick-hash")
                ]
            ),
            Ok(vec![
                Resolved::Type(0),
                Resolved::Member { owner: 0, field: 1 },
                Resolved::Member { owner: 0, field: 0 },
                Resolved::Thunk(0)
            ])
        );
    }

    #[test]
    fn native_only_refusal_names_the_path_even_with_no_generated_entries() {
        let table = Table {
            native_only: &["GPU::draw"],
            ..Table::EMPTY
        };
        let refusal = resolve(&table, "test", &[("GPU::draw", "hash")]).unwrap_err();
        assert_eq!(
            refusal,
            crate::Refusal::MissingInternal {
                path: "GPU::draw".into(),
                file: "crates/horizon-mod-api-gen/wasm-internals/test.txt".into(),
                native_only: true,
            }
        );
        assert_eq!(refusal.to_string(), "native only: GPU::draw");
        assert_eq!(
            resolve(&table, "test", &[("Game::unlisted", "hash")]),
            Err(crate::Refusal::MissingInternal {
                path: "Game::unlisted".into(),
                file: "crates/horizon-mod-api-gen/wasm-internals/test.txt".into(),
                native_only: false,
            })
        );
    }

    #[test]
    fn declared_paths_resolve_by_path_then_hash() {
        assert_eq!(
            resolve(
                &TABLE,
                "test",
                &[("Game", "game-hash"), ("Game::kick", "kick-hash")]
            ),
            Ok(vec![Resolved::Type(0), Resolved::Thunk(0)])
        );
        assert_eq!(
            resolve(&TABLE, "test", &[("Game", "old")]),
            Err(crate::Refusal::ChangedInternal {
                path: "Game".into()
            })
        );
        for path in ["Missing", "Vec<Player>"] {
            assert_eq!(
                resolve(&TABLE, "test", &[(path, "")]),
                Err(crate::Refusal::MissingInternal {
                    path: path.into(),
                    file: "crates/horizon-mod-api-gen/wasm-internals/test.txt".into(),
                    native_only: false,
                })
            );
        }
        assert_eq!(
            resolve(&Table::EMPTY, "test", &[("Game", "game-hash")]),
            Err(crate::Refusal::NoInternalsTable)
        );
        assert_eq!(resolve(&Table::EMPTY, "test", &[]), Ok(vec![]));
    }
}
