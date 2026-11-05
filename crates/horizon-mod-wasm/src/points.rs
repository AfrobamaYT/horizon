//! Wasm mods listening at the game's points (#589): a `[wasm]` package's `[[native.injection]]`
//! entries name the places its `mod.wasm` fills. At load each is checked against this game's
//! points and against the mod's own listeners, then the mod registers in the points crate's one
//! list of listeners, exactly as a native mod does: its places fire it through
//! `mod_points::wire::Forwarded`, as hook calls within its group's limits.
use crate::Refusal;
use horizon_mods::manifest::{InjectionKind, Manifest};
use mod_api::__private::wasm_host::{CallHost, Exports};
use mod_points::__private::wire::{Forward, Forwarded, HostKind, PointHost, Record, Value};
use mod_points::PointId;
use mod_points::places::{IDS, NAMES, SHAPES, TARGETS};
use std::collections::BTreeSet;

type Subscribed = wasmtime::component::TypedFunc<(), (Vec<(String, String)>,)>;
type Fire = wasmtime::component::TypedFunc<(u32, HostKind, Vec<Value>), (Vec<Value>,)>;

/// One wasm mod's listeners: its `listeners` export, and the places it fills in the order the
/// mod named them (a place's index there is the one `fire` takes).
struct Listeners {
    exports: Exports,
    fire: Fire,
    places: &'static [Option<PointId>],
}

impl Forward for Listeners {
    fn subscribed(&self) -> &'static [Option<PointId>] {
        self.places
    }

    fn forward(&self, place: PointId, host: PointHost<'_>, record: &mut dyn Record) {
        let Some(index) = self.places.iter().position(|p| *p == Some(place)) else {
            return;
        };
        let (host, wire) = match host {
            PointHost::Read(h) => (CallHost::Read(h), HostKind::Read),
            PointHost::Server(h) | PointHost::Side(mod_api::HostSide::Server(h)) => {
                (CallHost::Server(h), HostKind::Server)
            }
            PointHost::Client(h) | PointHost::Side(mod_api::HostSide::Client(h)) => {
                (CallHost::Client(h), HostKind::Client)
            }
        };
        let fields = record.lower();
        let answer = self.exports.call(host, |store| {
            Ok(self.fire.call(store, (index as u32, wire, fields))?.0)
        });
        let written = match answer {
            Some(fields) => record.write_back(fields),
            None if self.exports.stopped() => {
                mod_points::stop(self.exports.id());
                return;
            }
            // Refused (busy, or a call back into its own group): the place runs as if it
            // weren't listened at, as a hook's would.
            None => return,
        };
        if let Err(why) = written {
            log::error!(
                "wasm mod {} stopped listening at points: at {}, {why}",
                self.exports.id(),
                NAMES[place as usize]
            );
            mod_points::stop(self.exports.id());
        }
    }
}

/// The place a manifest's injection names: its `Points` method, `<method>` at the head and
/// `<method>_return` at the return, which must be a place of this game at the same function.
fn declared_place(target: &str, method: &str, kind: InjectionKind) -> Result<usize, Refusal> {
    let name = match kind {
        InjectionKind::Return => format!("{method}_return"),
        _ => method.to_owned(),
    };
    NAMES
        .iter()
        .position(|place| *place == name)
        .filter(|&place| TARGETS[place] == target)
        .ok_or_else(|| Refusal::NoSuchPoint {
            point: format!("{target} ({name})"),
        })
}

/// Checks a loaded wasm mod's listeners against its manifest and this game's points, and
/// registers them. A mod whose manifest lists no injections listens nowhere and is left alone.
pub(crate) fn listen(manifest: &Manifest, exports: &Exports) -> Result<(), Refusal> {
    let Some(wasm) = &manifest.wasm else {
        return Ok(());
    };
    if wasm.points().is_empty() {
        return Ok(());
    }
    let mut declared = BTreeSet::new();
    for injection in wasm.points() {
        // The manifest admits only `trait_method` injections with a `target` beside `[wasm]`.
        let (Some(target), Some(method)) = (&injection.target, &injection.trait_method) else {
            continue;
        };
        declared.insert(declared_place(target, method, injection.kind)?);
    }
    let interface = format!(
        "{}-listeners",
        horizon_mods::manifest::export_prefix(&manifest.id)
    );
    let subscribed: Subscribed = exports
        .func(&interface, "subscribed")
        .map_err(Refusal::Load)?;
    let fire: Fire = exports.func(&interface, "fire").map_err(Refusal::Load)?;
    let listed = exports
        .load(|store| Ok(subscribed.call(store, ())?.0))
        .ok_or_else(|| {
            Refusal::Points("its listeners didn't answer which places they fill".into())
        })?;
    let mut places = Vec::with_capacity(listed.len());
    for (name, shape) in &listed {
        let Some(place) = NAMES.iter().position(|place| place == name) else {
            return Err(Refusal::NoSuchPoint {
                point: name.clone(),
            });
        };
        if !declared.remove(&place) {
            return Err(Refusal::UndeclaredPoint {
                point: name.clone(),
            });
        }
        if SHAPES[place] != shape {
            return Err(Refusal::ChangedPoint {
                point: name.clone(),
                built: shape.clone(),
                game: SHAPES[place].to_owned(),
            });
        }
        places.push(Some(IDS[place]));
    }
    if let Some(&place) = declared.first() {
        return Err(Refusal::UnfilledPoint {
            point: NAMES[place].to_owned(),
        });
    }
    let listeners = Listeners {
        exports: exports.clone(),
        fire,
        places: Box::leak(places.into_boxed_slice()),
    };
    let points: &'static Forwarded = Box::leak(Box::new(Forwarded(Box::new(listeners))));
    mod_points::register(exports.id(), points).map_err(|e| Refusal::Points(e.to_string()))
}
