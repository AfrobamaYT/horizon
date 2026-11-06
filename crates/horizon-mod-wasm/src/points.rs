//! Wasm mods listening at the game's points (#589): a `[wasm]` package's `[[inject]]`
//! entries name the places its `mod.wasm` fills. At load each is checked against this game's
//! points and against the mod's own listeners, then the mod registers in the points crate's one
//! list of listeners, exactly as a native mod does: its places fire it through
//! `mod_points::wire::forwarded`, as hook calls within its group's limits.
use crate::Refusal;
use horizon_mods::manifest::{
    InjectionKind, Manifest,
    inject::{Access, Inject, Kind},
};
use mod_api::__private::wasm_host::{CallHost, Exports};
use mod_points::__private::wire::{Forward, HostKind, PointHost, Record, Value, forwarded};
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
            PointHost::None => (CallHost::None, HostKind::None),
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

/// Resolve the API function slots currently supported by the guest export path.
fn declared_place(inject: &Inject) -> Result<usize, Refusal> {
    let kind = match (inject.access, inject.kind, inject.priority) {
        (Access::Api, Kind::Head, 0) => InjectionKind::Head,
        (Access::Api, Kind::Return, 0) => InjectionKind::Return,
        _ => {
            return Err(Refusal::Points(format!(
                "{}: wasm injects currently require API head/return with priority 0",
                inject.id
            )));
        }
    };
    target_place(&inject.at, kind)
}

/// The point at a target's head or return, shared by API and raw listener validation.
pub(crate) fn target_place(target: &str, kind: InjectionKind) -> Result<usize, Refusal> {
    let missing = || Refusal::NoSuchPoint {
        point: target.into(),
    };
    let head = TARGETS
        .iter()
        .position(|t| *t == target)
        .ok_or_else(missing)?;
    match kind {
        InjectionKind::Return => Some(head + 1)
            .filter(|&back| TARGETS.get(back) == Some(&target))
            .ok_or_else(missing),
        _ => Ok(head),
    }
}

/// A wasm mod's checked listeners, registered once every native mod's are, in the installed order.
/// It owns them until then: one dropped unregistered (its mod refused after the check) lets its
/// group go.
pub struct Listening {
    exports: Exports,
    fire: Option<Fire>,
    places: Vec<Option<PointId>>,
    raw: Option<crate::raw::Listeners>,
}

impl Listening {
    /// Registers the listeners in the points crate's one list, which keeps them for good.
    pub fn register(self) -> Result<(), mod_points::RegisterError> {
        let id = self.exports.id();
        if let Some(fire) = self.fire {
            let listeners = Listeners {
                exports: self.exports,
                fire,
                places: Box::leak(self.places.into_boxed_slice()),
            };
            mod_points::register(id, Box::leak(forwarded(Box::new(listeners))))?;
        }
        if let Some(raw) = self.raw {
            raw.register()?;
        }
        Ok(())
    }
}

/// Checks a loaded wasm mod's listeners against its manifest and this game's points: the listeners
/// to register, none for a mod whose manifest lists no injections.
pub(crate) fn listen(manifest: &Manifest, exports: &Exports) -> Result<Option<Listening>, Refusal> {
    if manifest.wasm.is_none() {
        return Ok(None);
    }
    let raw = crate::raw::listen(manifest, exports)?;
    if manifest.inject.is_empty() {
        return Ok(raw.map(|raw| Listening {
            exports: exports.clone(),
            fire: None,
            places: vec![],
            raw: Some(raw),
        }));
    }
    let mut declared = BTreeSet::new();
    for inject in &manifest.inject {
        if !declared.insert(declared_place(inject)?) {
            return Err(Refusal::Points(format!(
                "{}: multiple injects at one wasm API place await the function registry",
                inject.id
            )));
        }
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
    Ok(Some(Listening {
        exports: exports.clone(),
        fire: Some(fire),
        places,
        raw,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TICK: &str = "horizon_client::game::Game<'_>::tick";

    #[test]
    fn an_injection_names_the_place_its_kind_and_method_say() {
        let mut inject = Inject {
            id: "tick".into(),
            at: TICK.into(),
            kind: Kind::Head,
            with: "any_function".into(),
            access: Access::Api,
            priority: 0,
            hash: "signature".into(),
            call: None,
            nth: None,
            ctx: None,
        };
        assert_eq!(NAMES[declared_place(&inject).unwrap()], "tick");
        inject.kind = Kind::Return;
        assert_eq!(NAMES[declared_place(&inject).unwrap()], "tick_return");
        for (access, kind, priority) in [
            (Access::Internals, Kind::Head, 0),
            (Access::Api, Kind::WrapCall, 0),
            (Access::Api, Kind::Head, 1),
        ] {
            inject.access = access;
            inject.kind = kind;
            inject.priority = priority;
            assert!(matches!(declared_place(&inject), Err(Refusal::Points(_))));
        }
        inject.access = Access::Api;
        inject.kind = Kind::Head;
        inject.priority = 0;
        inject.at = "game::gone".into();
        assert!(matches!(
            declared_place(&inject),
            Err(Refusal::NoSuchPoint { .. })
        ));
    }
}
