//! One call's view of the game's internals (#589 W): where a wasm listener's nodes are, what it
//! wrote, and the game functions it called. The game's generated table does each read, write and
//! call; this keeps the rules that make them sound.
//!
//! - A node is a path from one of the call's roots, never a kept pointer: the pointer it resolved
//!   to is cached, and the cache is dropped by the only thing that can move the game's memory
//!   while the mod runs, a call into a game function.
//! - Writes go to the call's journal, and the mod's own reads see them. They apply, in order,
//!   before a game function runs and when the listener returns; a trap drops them.
//! - A reference a game function returns is a root that lasts until the next call.
use crate::journal::{Journal, Path, Step};
use mod_points::internals::{Arg, Op, Refusal, Reply, Table, Value};
use std::{cell::Cell, collections::BTreeSet};

/// Where a root is.
#[derive(Clone, Copy, Debug)]
pub struct Root {
    pub ptr: *mut (),
    /// Its type, an index into the table's types.
    pub ty: u32,
    pub writable: bool,
    /// A slot's receiver, whose fields require this mod's explicit declarations.
    pub receiver: bool,
    /// The game function call that returned it: valid only until the next one. `None` for the
    /// call's own roots, valid for the whole call.
    pub born: Option<u64>,
}

/// A node of the game's state, for one call.
#[derive(Debug)]
pub struct Node {
    pub path: Path<Value>,
    /// Receiver nodes expose only the fields declared by the current mod.
    pub(crate) restricted: bool,
    /// Where it last resolved, and in which generation.
    at: Cell<Option<(u64, Resolved)>>,
}

impl Node {
    fn new(path: Path<Value>, restricted: bool) -> Self {
        Self {
            path,
            restricted,
            at: Cell::new(None),
        }
    }
}

/// A node resolved: its pointer, its type and whether it may be written.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resolved {
    pub ptr: *mut (),
    pub ty: u32,
    pub writable: bool,
}

/// A step's answer: a node, or an element's copy.
#[derive(Debug)]
pub enum Reached {
    Node(Node),
    Value(Value),
}

/// One call's frame.
pub struct Frame<'t> {
    table: &'t Table,
    roots: Vec<Root>,
    journal: Journal<Value>,
    allowed_members: BTreeSet<(u32, u32)>,
    /// Bumped by every game function call: every cached pointer is stale after it.
    generation: u64,
    cache_generation: Cell<u64>,
    journal_cells: usize,
    journal_entries: usize,
}

fn gone(what: &str) -> Refusal {
    Refusal::Gone(what.to_owned())
}

fn not_here(what: &str) -> Refusal {
    Refusal::NotHere(what.to_owned())
}

/// Validates the size of an untrusted value once, before retaining or decoding it.
pub(crate) fn value_cells(value: &Value) -> Result<usize, Refusal> {
    let mut pending = vec![value];
    let mut cells = 0;
    let mut bytes = 0;
    while let Some(value) = pending.pop() {
        cells += 1;
        match value {
            Value::Str(s) => bytes += s.len(),
            Value::Seq(v) | Value::Rec(v) | Value::Var(_, v) => pending.extend(v),
            _ => {}
        }
        if cells > 65_536 || bytes > 1_048_576 {
            return Err(not_here("internals budget"));
        }
    }
    Ok(cells)
}

impl<'t> Frame<'t> {
    /// A frame over `roots`: the slot's own borrows for this call, each valid until it returns.
    ///
    /// # Safety
    ///
    /// Each root points to a live value of its type in `table`, borrowed mutably (or shared, when
    /// it isn't `writable`) for as long as the frame lives, and nothing else touches them meanwhile.
    pub unsafe fn new(table: &'t Table, roots: Vec<Root>) -> Self {
        Self {
            table,
            roots,
            journal: Journal::default(),
            allowed_members: BTreeSet::new(),
            generation: 0,
            cache_generation: Cell::new(0),
            journal_cells: 0,
            journal_entries: 0,
        }
    }

    /// Installs the current mod's field permissions once at the call boundary.
    pub(crate) fn allow_members(&mut self, members: impl IntoIterator<Item = (u32, u32)>) {
        self.allowed_members = members.into_iter().collect();
    }

    /// The call's root `n`.
    pub fn root(&self, n: u32) -> Result<Node, Refusal> {
        let root = self.live_root(n)?;
        Ok(Node::new(
            Path {
                root: n,
                steps: Vec::new(),
            },
            root.receiver,
        ))
    }

    fn live_root(&self, n: u32) -> Result<Root, Refusal> {
        let root = *self
            .roots
            .get(n as usize)
            .ok_or_else(|| gone("no such root"))?;
        match root.born {
            Some(born) if born != self.generation => Err(gone(
                "a reference a game function returned before its next call",
            )),
            _ => Ok(root),
        }
    }

    fn op(&self, at: Resolved, op: Op<'_>) -> Result<Reply, Refusal> {
        let ty = self
            .table
            .types
            .get(at.ty as usize)
            .ok_or_else(|| gone("no such type"))?;
        if !self.raw_step(at.ty, &op) {
            self.cache_generation.set(self.cache_generation.get() + 1);
        }
        // SAFETY: `at` was resolved in this generation from a root by the table's own steps, so it
        // points to a live value of type `at.ty`, writable only when `at.writable`.
        unsafe { (ty.node)(at.ptr, at.writable, op) }
    }

    fn raw_step(&self, ty: u32, op: &Op<'_>) -> bool {
        let ty = &self.table.types[ty as usize];
        match op {
            Op::Field(n) => ty.raw_fields.contains(n),
            Op::Index(_) => ty.raw_index,
            _ => false,
        }
    }

    fn disjoint(&self, left: &Node, right: &Node) -> Result<bool, Refusal> {
        let l = self.live_root(left.path.root)?;
        let r = self.live_root(right.path.root)?;
        if l.born.is_some() || r.born.is_some() {
            return Ok(false);
        }
        if left.path.root != right.path.root {
            return Ok(true);
        }
        let mut prefix = Path {
            root: left.path.root,
            steps: Vec::new(),
        };
        for (a, b) in left.path.steps.iter().zip(&right.path.steps) {
            let at = self.walk(&prefix)?;
            let op = match a {
                Step::Field(n) => Op::Field(*n),
                Step::Index(i) => Op::Index(*i),
                Step::Key(k) => Op::Key(k),
            };
            if !self.raw_step(at.ty, &op) {
                return Ok(false);
            }
            if a != b {
                return Ok(matches!(
                    (a, b),
                    (Step::Field(_), Step::Field(_)) | (Step::Index(_), Step::Index(_))
                ));
            }
            prefix.steps.push(a.clone());
        }
        Ok(false)
    }

    /// Where `path` leads now, step by step from its root.
    fn walk(&self, path: &Path<Value>) -> Result<Resolved, Refusal> {
        let root = self.live_root(path.root)?;
        let mut at = Resolved {
            ptr: root.ptr,
            ty: root.ty,
            writable: root.writable,
        };
        for step in &path.steps {
            let op = match step {
                Step::Field(field) => Op::Field(*field),
                Step::Index(i) => Op::Index(*i),
                Step::Key(key) => Op::Key(key),
            };
            at = match self.op(at, op)? {
                Reply::Node(ptr, ty, writable) => Resolved { ptr, ty, writable },
                // An element that became a copy, or another case's field: the path no longer leads
                // to a node.
                _ => return Err(gone("a path that no longer leads to a node")),
            };
        }
        Ok(at)
    }

    /// Where `node` is, from its cache when nothing has moved since.
    pub fn resolve(&self, node: &Node) -> Result<Resolved, Refusal> {
        if let Some((generation, at)) = node.at.get()
            && generation == self.cache_generation.get()
        {
            return Ok(at);
        }
        let at = self.walk(&node.path)?;
        node.at.set(Some((self.cache_generation.get(), at)));
        Ok(at)
    }

    fn field_access(&self, node: &Node, field: u32) -> Result<Resolved, Refusal> {
        let at = self.resolve(node)?;
        if node.restricted && !self.allowed_members.contains(&(at.ty, field)) {
            return Err(not_here("undeclared field"));
        }
        Ok(at)
    }

    fn step(&self, node: &Node, step: Step<Value>) -> Result<Reached, Refusal> {
        let at = match &step {
            Step::Field(field) => self.field_access(node, *field)?,
            _ => self.resolve(node)?,
        };
        let reply = match &step {
            Step::Field(field) => self.op(at, Op::Field(*field))?,
            Step::Index(i) => self.op(at, Op::Index(*i))?,
            Step::Key(key) => self.op(at, Op::Key(key))?,
        };
        let mut path = node.path.clone();
        path.steps.push(step);
        Ok(match reply {
            Reply::Node(ptr, ty, writable) => {
                let next = Node::new(path, false);
                next.at.set(Some((
                    self.cache_generation.get(),
                    Resolved { ptr, ty, writable },
                )));
                Reached::Node(next)
            }
            Reply::Value(value) => Reached::Value(value),
            Reply::Done => return Err(not_here("a step that answered nothing")),
        })
    }

    /// A handle-typed field of `node`.
    pub fn field(&self, node: &Node, field: u32) -> Result<Node, Refusal> {
        match self.step(node, Step::Field(field))? {
            Reached::Node(node) => Ok(node),
            Reached::Value(_) => Err(not_here("a copy, read with get")),
        }
    }

    /// Element `i` of a list, array or slice: its node, or its copy for a scalar.
    pub fn index(&self, node: &Node, i: u64) -> Result<Reached, Refusal> {
        self.step(node, Step::Index(i))
    }

    /// A map's entry by `key`: its node, or its copy for a scalar.
    pub fn key(&self, node: &Node, key: Value) -> Result<Reached, Refusal> {
        fn canonical(value: Value) -> Value {
            match value {
                Value::S(n) if n >= 0 => Value::U(n as u64),
                Value::Rec(v) => Value::Rec(v.into_iter().map(canonical).collect()),
                Value::Seq(v) => Value::Seq(v.into_iter().map(canonical).collect()),
                Value::Var(c, v) => Value::Var(c, v.into_iter().map(canonical).collect()),
                v => v,
            }
        }
        self.step(node, Step::Key(canonical(key)))
    }

    /// A copy-typed field of `node`, as this call last wrote it, else as the game has it.
    pub fn get(&self, node: &Node, field: u32) -> Result<Value, Refusal> {
        let at = self.field_access(node, field)?;
        if let Some(value) = self.journal.get(&node.path, field) {
            return Ok(value.clone());
        }
        match self.op(at, Op::Get(field))? {
            Reply::Value(value) => Ok(value),
            _ => Err(not_here("a field that isn't a copy")),
        }
    }

    /// Writes a copy-typed field of `node` when the listener returns, or before its next game
    /// function call.
    pub fn set(&mut self, node: &Node, field: u32, value: Value) -> Result<(), Refusal> {
        let at = self.field_access(node, field)?;
        if !at.writable {
            return Err(not_here("a read-only node"));
        }
        let mut cells = value_cells(&value)?;
        for step in &node.path.steps {
            if let Step::Key(key) = step {
                cells += value_cells(key)?;
            }
        }
        if self.journal_entries >= 4096 || self.journal_cells + cells > 65_536 {
            return Err(not_here("internals budget"));
        }
        self.op(at, Op::Check(field, &value))?;
        self.journal_entries += 1;
        self.journal_cells += cells;
        self.journal.set(node.path.clone(), field, value);
        Ok(())
    }

    /// Whether the call wrote under `node`: its whole-node reads would miss those writes.
    fn written_under(&self, node: &Node) -> bool {
        self.journal
            .any(|path| path.root == node.path.root && path.steps.starts_with(&node.path.steps))
    }

    /// The whole of `node`, when its type is a copy.
    pub fn read(&self, node: &Node) -> Result<Value, Refusal> {
        if node.restricted {
            return Err(not_here("undeclared field"));
        }
        if self.written_under(node) {
            return Err(not_here("a node this call wrote under: read its fields"));
        }
        match self.op(self.resolve(node)?, Op::Read)? {
            Reply::Value(value) => Ok(value),
            _ => Err(not_here("a node that isn't a copy")),
        }
    }

    /// A list's, a map's or a set's length.
    pub fn len(&self, node: &Node) -> Result<u64, Refusal> {
        match self.op(self.resolve(node)?, Op::Len)? {
            Reply::Value(Value::U(n)) => Ok(n),
            _ => Err(not_here("a node with no length")),
        }
    }

    /// An enum's or an `Option`'s case.
    pub fn case(&self, node: &Node) -> Result<u32, Refusal> {
        match self.op(self.resolve(node)?, Op::Case)? {
            Reply::Value(Value::U(n)) => u32::try_from(n).map_err(|_| not_here("an unknown case")),
            _ => Err(not_here("a node with no case")),
        }
    }

    /// Applies the call's writes, in order: before a game function call, and when the listener
    /// returns. Answers the writes whose path led nowhere, each with its field and why.
    pub fn apply(&mut self) -> Vec<(Path<Value>, u32, Refusal)> {
        self.journal_entries = 0;
        self.journal_cells = 0;
        let mut refused = Vec::new();
        let mut journal = std::mem::take(&mut self.journal);
        let gone = journal.apply(|path, field, value| {
            let result = self.walk(path).and_then(|at| {
                if at.writable {
                    self.op(at, Op::Set(field, value))
                } else {
                    Err(not_here("a read-only node"))
                }
            });
            result.map(drop).map_err(|why| {
                refused.push((path.clone(), field, why));
                crate::journal::Gone
            })
        });
        debug_assert_eq!(gone.len(), refused.len());
        refused
    }

    /// Calls the declared function `function` on `receiver` with `args`, after applying the
    /// call's writes. A reference it returns is a new root, valid until the next call. Answers
    /// what it returned, and the writes that couldn't apply.
    pub fn call(
        &mut self,
        function: u32,
        receiver: Option<&Node>,
        args: Vec<Call>,
    ) -> Result<(Reached, Vec<(Path<Value>, u32, Refusal)>), Refusal> {
        let table = self.table;
        let thunk = table
            .thunks
            .get(function as usize)
            .ok_or_else(|| gone("no such function"))?;
        if args.len() != thunk.args.len() {
            return Err(Refusal::Changed("an argument count".into()));
        }
        // Rust separates every mutable borrow from both mutable and shared arguments. Paths
        // prove separation without trusting cached pointers or aliases returned by thunks.
        let borrowed = receiver
            .map(|receiver| (receiver, thunk.writes & 1 != 0))
            .into_iter()
            .chain(args.iter().enumerate().filter_map(|(index, arg)| {
                if let Call::Node(node) = arg {
                    Some((*node, thunk.writes >> (index + 1) & 1 != 0))
                } else {
                    None
                }
            }));
        for (index, (left, left_written)) in borrowed.clone().enumerate() {
            for (right, right_written) in borrowed.clone().skip(index + 1) {
                if !left_written && !right_written {
                    continue;
                }
                let disjoint = self.disjoint(left, right)?;
                if !disjoint {
                    return Err(not_here("arguments overlap"));
                }
            }
        }
        // The writes go first, as if the function ran after the listener's code before it; then
        // every node resolves again, since a write may have moved what it points into.
        let wrote = !self.journal.is_empty();
        let refused = self.apply();
        if wrote {
            self.generation += 1;
            self.cache_generation.set(self.cache_generation.get() + 1);
        }
        let check = |node: &Node, ty: u32, written: bool| -> Result<*mut (), Refusal> {
            let at = self.resolve(node)?;
            if at.ty != ty {
                return Err(Refusal::Changed("a node of another type".into()));
            }
            if written && !at.writable {
                return Err(not_here("a read-only node where the function writes"));
            }
            Ok(at.ptr)
        };
        let this = match (receiver, thunk.receiver) {
            (Some(node), Some(ty)) => Some(check(node, ty, thunk.writes & 1 != 0)?),
            (None, None) => None,
            _ => return Err(Refusal::Changed("a receiver".into())),
        };
        let mut lowered = Vec::with_capacity(args.len());
        for (i, (arg, ty)) in args.into_iter().zip(thunk.args).enumerate() {
            let writes = thunk.writes >> (i + 1) & 1 != 0;
            lowered.push(match (arg, ty) {
                (Call::Value(value), None) => Arg::Value(value),
                (Call::Node(node), Some(ty)) => {
                    let ptr = check(node, *ty, writes)?;
                    Arg::Node(ptr)
                }
                _ => {
                    return Err(Refusal::Changed(
                        "a node where a value goes, or the other way".into(),
                    ));
                }
            });
        }
        // SAFETY: each node was resolved in this generation to a live value of the thunk's
        // parameter type, writable where the thunk writes, and each written one is disjoint
        // from every other node parameter.
        let reply = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            (thunk.call)(this, lowered)
        }))
        .unwrap_or_else(|panic| {
            log::error!("wasm internals thunk panicked: {}", thunk.path);
            std::panic::resume_unwind(panic)
        });
        self.generation += 1;
        self.cache_generation.set(self.cache_generation.get() + 1);
        let reply = reply?;
        let reached = match reply {
            Reply::Node(ptr, ty, writable) => {
                self.roots.push(Root {
                    ptr,
                    ty,
                    writable,
                    receiver: false,
                    born: Some(self.generation),
                });
                Reached::Node(Node::new(
                    Path {
                        root: (self.roots.len() - 1) as u32,
                        steps: Vec::new(),
                    },
                    false,
                ))
            }
            Reply::Value(value) => Reached::Value(value),
            Reply::Done => Reached::Value(Value::None),
        };
        Ok((reached, refused))
    }
}

/// A game function's argument from the mod: a copy, or one of its nodes.
pub enum Call<'n> {
    Value(Value),
    Node(&'n Node),
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use mod_points::internals::{Member, Thunk, Type};

    /// A tiny game: its players, each with health, and a flag.
    pub(crate) struct Game {
        players: Vec<Player>,
        flag: bool,
    }
    struct Player {
        health: i64,
    }

    const GAME: u32 = 0;
    const PLAYERS: u32 = 1;
    const PLAYER: u32 = 2;

    // The table the generator would make for these, by hand.
    unsafe fn game(p: *mut (), w: bool, op: Op<'_>) -> Result<Reply, Refusal> {
        let game = p as *mut Game;
        unsafe {
            match op {
                Op::Field(0) => Ok(Reply::Node(&raw mut (*game).players as *mut (), PLAYERS, w)),
                Op::Check(1, Value::B(_)) => Ok(Reply::Done),
                Op::Get(1) => Ok(Reply::Value(Value::B((*game).flag))),
                Op::Set(1, Value::B(b)) => {
                    (*game).flag = b;
                    Ok(Reply::Done)
                }
                _ => Err(not_here("not on this node")),
            }
        }
    }
    unsafe fn players(p: *mut (), w: bool, op: Op<'_>) -> Result<Reply, Refusal> {
        let players = unsafe { &mut *(p as *mut Vec<Player>) };
        match op {
            Op::Len => Ok(Reply::Value(Value::U(players.len() as u64))),
            Op::Index(i) if (i as usize) < players.len() => Ok(Reply::Node(
                unsafe { players.as_mut_ptr().add(i as usize) }.cast(),
                PLAYER,
                w,
            )),
            _ => Err(not_here("not on this node")),
        }
    }
    unsafe fn player(p: *mut (), _: bool, op: Op<'_>) -> Result<Reply, Refusal> {
        let player = unsafe { &mut *(p as *mut Player) };
        match op {
            Op::Check(0, Value::S(_)) => Ok(Reply::Done),
            Op::Get(0) => Ok(Reply::Value(Value::S(player.health))),
            Op::Set(0, Value::S(n)) => {
                player.health = n;
                Ok(Reply::Done)
            }
            Op::Read => Ok(Reply::Value(Value::Rec(vec![Value::S(player.health)]))),
            _ => Err(not_here("not on this node")),
        }
    }
    /// `Game::kick(&mut self) -> usize`: drops the last player, answers how many are left.
    unsafe fn kick(this: Option<*mut ()>, _: Vec<Arg>) -> Result<Reply, Refusal> {
        let game = unsafe { &mut *(this.unwrap() as *mut Game) };
        game.players.pop();
        Ok(Reply::Value(Value::U(game.players.len() as u64)))
    }
    /// `Game::first(&mut self) -> &mut Player`.
    unsafe fn first(this: Option<*mut ()>, _: Vec<Arg>) -> Result<Reply, Refusal> {
        let game = unsafe { &mut *(this.unwrap() as *mut Game) };
        Ok(Reply::Node(
            &raw mut game.players[0] as *mut (),
            PLAYER,
            true,
        ))
    }
    /// `heal(a: &mut Player, b: &mut Player)`.
    unsafe fn heal(_: Option<*mut ()>, args: Vec<Arg>) -> Result<Reply, Refusal> {
        for arg in args {
            let Arg::Node(p) = arg else { unreachable!() };
            unsafe { (*(p as *mut Player)).health = 20 };
        }
        Ok(Reply::Done)
    }

    unsafe fn heal_player(this: Option<*mut ()>, _: Vec<Arg>) -> Result<Reply, Refusal> {
        let player = unsafe { &mut *this.unwrap().cast::<Player>() };
        player.health = 20;
        Ok(Reply::Done)
    }

    static TYPES: [Type; 3] = [
        Type {
            path: "Game",
            hash: "game-hash",
            node: game,
            raw_fields: &[0, 1],
            raw_index: true,
        },
        Type {
            path: "Vec<Player>",
            hash: "",
            node: players,
            raw_fields: &[0, 1],
            raw_index: true,
        },
        Type {
            path: "Player",
            hash: "player-hash",
            node: player,
            raw_fields: &[0, 1],
            raw_index: true,
        },
    ];
    unsafe fn inspect_player(_: Option<*mut ()>, _: Vec<Arg>) -> Result<Reply, Refusal> {
        panic!("an overlapping receiver and argument must be refused before the thunk");
    }

    static THUNKS: [Thunk; 5] = [
        Thunk {
            path: "Game::kick",
            hash: "kick-hash",
            receiver: Some(GAME),
            args: &[],
            writes: 1,
            call: kick,
        },
        Thunk {
            path: "Game::first",
            hash: "first-hash",
            receiver: Some(GAME),
            args: &[],
            writes: 1,
            call: first,
        },
        Thunk {
            path: "heal",
            hash: "heal-hash",
            receiver: None,
            args: &[Some(PLAYER), Some(PLAYER)],
            writes: 0b110,
            call: heal,
        },
        Thunk {
            path: "Player::heal",
            hash: "heal-player-hash",
            receiver: Some(PLAYER),
            args: &[],
            writes: 1,
            call: heal_player,
        },
        Thunk {
            path: "Game::inspect_player",
            hash: "inspect-player-hash",
            receiver: Some(GAME),
            args: &[Some(PLAYER)],
            writes: 1,
            call: inspect_player,
        },
    ];
    pub(crate) static TABLE: Table = Table {
        types: &TYPES,
        thunks: &THUNKS,
        native_only: &[],
        places: &[],
        members: &[
            Member {
                path: "Game::flag",
                hash: "flag-hash",
                owner: GAME,
                field: 1,
            },
            // A field and a method may share a Rust path; their hashes distinguish them.
            Member {
                path: "Game::kick",
                hash: "field-kick-hash",
                owner: GAME,
                field: 0,
            },
        ],
    };

    pub(crate) fn world() -> Game {
        Game {
            players: (0..3).map(|i| Player { health: 10 + i }).collect(),
            flag: false,
        }
    }

    pub(crate) fn frame(game: &mut Game) -> Frame<'static> {
        let root = Root {
            ptr: game as *mut Game as *mut (),
            ty: GAME,
            writable: true,
            receiver: true,
            born: None,
        };
        // SAFETY: `game` outlives the frame in every test, and only the frame touches it.
        let mut frame = unsafe { Frame::new(&TABLE, vec![root]) };
        frame.allow_members([(GAME, 0), (GAME, 1)]);
        frame
    }

    fn node(reached: Reached) -> Node {
        match reached {
            Reached::Node(node) => node,
            Reached::Value(value) => panic!("a copy: {value:?}"),
        }
    }

    #[test]
    fn writes_wait_for_the_return_and_the_listener_reads_its_own() {
        let mut game = world();
        let mut frame = frame(&mut game);
        let root = frame.root(0).unwrap();
        let players = frame.field(&root, 0).unwrap();
        let second = node(frame.index(&players, 1).unwrap());
        frame.set(&second, 0, Value::S(3)).unwrap();
        frame.set(&root, 1, Value::B(true)).unwrap();
        assert_eq!(frame.get(&second, 0).unwrap(), Value::S(3));
        assert!(
            frame.read(&second).is_err(),
            "a whole read would miss the write"
        );
        assert!(frame.apply().is_empty());
        drop(frame);
        assert_eq!(game.players[1].health, 3);
        assert!(game.flag);
    }

    #[test]
    fn a_trapped_listener_changes_nothing() {
        let mut game = world();
        let mut frame = frame(&mut game);
        let root = frame.root(0).unwrap();
        frame.set(&root, 1, Value::B(true)).unwrap();
        // The runtime drops the frame unapplied when the listener traps.
        drop(frame);
        assert!(!game.flag);
    }

    #[test]
    fn a_game_function_sees_earlier_writes_and_moves_every_node() {
        let mut game = world();
        let mut frame = frame(&mut game);
        let root = frame.root(0).unwrap();
        let players = frame.field(&root, 0).unwrap();
        let last = node(frame.index(&players, 2).unwrap());
        frame.set(&last, 0, Value::S(1)).unwrap();
        // The write applies before the call; the call then removes that player.
        let (left, refused) = frame.call(0, Some(&root), Vec::new()).unwrap();
        assert!(refused.is_empty());
        assert!(matches!(left, Reached::Value(Value::U(2))));
        // The node's path now leads past the end: it is refused, never read out of bounds.
        assert!(matches!(frame.get(&last, 0), Err(Refusal::NotHere(_))));
        assert_eq!(frame.len(&players).unwrap(), 2);
        // A write to a place a call removed is refused at once, never applied elsewhere.
        let second = node(frame.index(&players, 1).unwrap());
        frame.set(&second, 0, Value::S(5)).unwrap();
        frame.call(0, Some(&root), Vec::new()).unwrap();
        assert!(matches!(
            frame.set(&second, 0, Value::S(6)),
            Err(Refusal::NotHere(_))
        ));
        assert!(frame.apply().is_empty());
        drop(frame);
        // The first write applied before the call that removed its player.
        assert_eq!(game.players.len(), 1);
        assert_eq!(game.players[0].health, 10);
    }

    #[test]
    fn a_returned_reference_lasts_until_the_next_call() {
        let mut game = world();
        let mut frame = frame(&mut game);
        let root = frame.root(0).unwrap();
        let (first, _) = frame.call(1, Some(&root), Vec::new()).unwrap();
        let first = node(first);
        assert_eq!(frame.get(&first, 0).unwrap(), Value::S(10));
        frame.set(&first, 0, Value::S(7)).unwrap();
        // The next call applies that write first, then the reference is gone.
        frame.call(0, Some(&root), Vec::new()).unwrap();
        assert!(matches!(frame.get(&first, 0), Err(Refusal::Gone(_))));
        drop(frame);
        assert_eq!(game.players[0].health, 7);
    }

    #[test]
    fn a_returned_reference_can_be_the_next_calls_receiver() {
        let mut game = world();
        let mut frame = frame(&mut game);
        let root = frame.root(0).unwrap();
        let first = node(frame.call(1, Some(&root), Vec::new()).unwrap().0);
        frame.call(3, Some(&first), Vec::new()).unwrap();
        assert!(matches!(frame.get(&first, 0), Err(Refusal::Gone(_))));
        drop(frame);
        assert_eq!(game.players[0].health, 20);
        assert_eq!(game.players[1].health, 11);
    }

    #[test]
    fn pending_writes_expire_a_returned_reference_before_the_next_call() {
        let mut game = world();
        let mut frame = frame(&mut game);
        let root = frame.root(0).unwrap();
        let first = node(frame.call(1, Some(&root), Vec::new()).unwrap().0);
        frame.set(&root, 1, Value::B(true)).unwrap();
        assert!(matches!(
            frame.call(3, Some(&first), Vec::new()),
            Err(Refusal::Gone(_))
        ));
        drop(frame);
        assert!(
            game.flag,
            "the pending journal still applies before the refused call"
        );
        assert_eq!(game.players[0].health, 10, "the thunk did not run");
    }

    #[test]
    fn a_call_refuses_the_wrong_type_and_one_place_written_twice() {
        let mut game = world();
        let mut frame = frame(&mut game);
        let root = frame.root(0).unwrap();
        let players = frame.field(&root, 0).unwrap();
        let a = node(frame.index(&players, 0).unwrap());
        let b = node(frame.index(&players, 1).unwrap());
        let wrong = frame.call(0, Some(&a), Vec::new());
        assert!(matches!(wrong, Err(Refusal::Changed(_))));
        let twice = frame.call(2, None, vec![Call::Node(&a), Call::Node(&a)]);
        assert!(matches!(twice, Err(Refusal::NotHere(_))));
        frame
            .call(2, None, vec![Call::Node(&a), Call::Node(&b)])
            .unwrap();
        drop(frame);
        assert_eq!(
            [
                game.players[0].health,
                game.players[1].health,
                game.players[2].health
            ],
            [20, 20, 12]
        );
    }

    #[test]
    fn a_receiver_and_its_child_refuse_before_applying_the_journal() {
        let mut game = world();
        let mut frame = frame(&mut game);
        let root = frame.root(0).unwrap();
        let players = frame.field(&root, 0).unwrap();
        let first = node(frame.index(&players, 0).unwrap());
        frame.set(&root, 1, Value::B(true)).unwrap();
        assert!(
            matches!(frame.call(4, Some(&root), vec![Call::Node(&first)]),
            Err(Refusal::NotHere(why)) if why == "arguments overlap")
        );
        drop(frame);
        assert!(
            !game.flag,
            "overlap refuses before even the pending set applies"
        );
    }

    #[test]
    fn a_returned_reference_beside_its_receiver_is_not_provably_disjoint() {
        let mut game = world();
        let mut frame = frame(&mut game);
        let root = frame.root(0).unwrap();
        let first = node(frame.call(1, Some(&root), Vec::new()).unwrap().0);
        assert!(
            matches!(frame.call(4, Some(&root), vec![Call::Node(&first)]),
            Err(Refusal::NotHere(why)) if why == "arguments overlap")
        );
    }

    #[test]
    fn two_disjoint_mutable_fields_of_one_root_are_allowed() {
        struct Fields {
            left: Player,
            right: Player,
        }
        unsafe fn fields(ptr: *mut (), writable: bool, op: Op<'_>) -> Result<Reply, Refusal> {
            // SAFETY: the sole root below is a live Fields and each field has type Player.
            let fields = ptr.cast::<Fields>();
            unsafe {
                match op {
                    Op::Field(0) => Ok(Reply::Node(
                        (&raw mut (*fields).left).cast(),
                        PLAYER,
                        writable,
                    )),
                    Op::Field(1) => Ok(Reply::Node(
                        (&raw mut (*fields).right).cast(),
                        PLAYER,
                        writable,
                    )),
                    _ => Err(not_here("not a field")),
                }
            }
        }
        static FIELD_TYPES: [Type; 3] = [
            Type {
                path: "Fields",
                hash: "fields-hash",
                node: fields,
                raw_fields: &[0, 1],
                raw_index: true,
            },
            Type {
                path: "Vec<Player>",
                hash: "",
                node: players,
                raw_fields: &[0, 1],
                raw_index: true,
            },
            Type {
                path: "Player",
                hash: "player-hash",
                node: player,
                raw_fields: &[0, 1],
                raw_index: true,
            },
        ];
        let table = Table {
            types: &FIELD_TYPES,
            ..TABLE
        };
        let mut fields = Fields {
            left: Player { health: 1 },
            right: Player { health: 2 },
        };
        // SAFETY: only this frame accesses the live Fields until it is dropped.
        let mut frame = unsafe {
            Frame::new(
                &table,
                vec![Root {
                    ptr: (&raw mut fields).cast(),
                    ty: 0,
                    writable: true,
                    receiver: false,
                    born: None,
                }],
            )
        };
        let root = frame.root(0).unwrap();
        let left = frame.field(&root, 0).unwrap();
        let right = frame.field(&root, 1).unwrap();
        frame
            .call(2, None, vec![Call::Node(&left), Call::Node(&right)])
            .unwrap();
        drop(frame);
        assert_eq!((fields.left.health, fields.right.health), (20, 20));
    }

    #[test]
    fn map_keys_cannot_prove_disjointness_and_integer_keys_share_writes() {
        unsafe fn map(p: *mut (), w: bool, op: Op<'_>) -> Result<Reply, Refusal> {
            let map = unsafe { &mut *p.cast::<std::collections::BTreeMap<u32, Player>>() };
            match op {
                Op::Key(k) => {
                    let key = mod_points::internals::int::<u32>(k.clone())?;
                    let value = map.get_mut(&key).ok_or_else(|| not_here("key"))?;
                    Ok(Reply::Node((value as *mut Player).cast(), PLAYER, w))
                }
                _ => Err(not_here("map")),
            }
        }
        let table = Table {
            types: Box::leak(Box::new([
                Type {
                    path: "map",
                    hash: "",
                    node: map,
                    raw_fields: &[],
                    raw_index: false,
                },
                Type {
                    path: "unused",
                    hash: "",
                    node: players,
                    raw_fields: &[],
                    raw_index: true,
                },
                Type {
                    path: "Player",
                    hash: "",
                    node: player,
                    raw_fields: &[],
                    raw_index: false,
                },
            ])),
            ..TABLE
        };
        let mut map = std::collections::BTreeMap::from([
            (1, Player { health: 4 }),
            (2, Player { health: 5 }),
        ]);
        let mut frame = unsafe {
            Frame::new(
                &table,
                vec![Root {
                    ptr: (&raw mut map).cast(),
                    ty: 0,
                    writable: true,
                    receiver: false,
                    born: None,
                }],
            )
        };
        let root = frame.root(0).unwrap();
        let signed = node(frame.key(&root, Value::S(1)).unwrap());
        let unsigned = node(frame.key(&root, Value::U(1)).unwrap());
        let other = node(frame.key(&root, Value::U(2)).unwrap());
        for b in [&unsigned, &other] {
            assert!(
                matches!(frame.call(2, None, vec![Call::Node(&signed), Call::Node(b)]), Err(Refusal::NotHere(why)) if why == "arguments overlap")
            );
        }
        frame.set(&signed, 0, Value::S(17)).unwrap();
        assert_eq!(frame.get(&unsigned, 0).unwrap(), Value::S(17));
    }

    #[test]
    fn reborrow_invalidates_sibling_caches_not_their_paths() {
        enum Choice {
            Players(Vec<Player>),
        }
        unsafe fn choice(p: *mut (), w: bool, op: Op<'_>) -> Result<Reply, Refusal> {
            match op {
                Op::Field(0) => {
                    let Choice::Players(v) = unsafe { &mut *p.cast::<Choice>() };
                    Ok(Reply::Node((v as *mut Vec<Player>).cast(), PLAYERS, w))
                }
                _ => Err(not_here("choice")),
            }
        }
        let table = Table {
            types: Box::leak(Box::new([
                Type {
                    path: "Choice",
                    hash: "",
                    node: choice,
                    raw_fields: &[],
                    raw_index: false,
                },
                Type {
                    path: "Vec",
                    hash: "",
                    node: players,
                    raw_fields: &[],
                    raw_index: true,
                },
                Type {
                    path: "Player",
                    hash: "",
                    node: player,
                    raw_fields: &[],
                    raw_index: false,
                },
            ])),
            ..TABLE
        };
        let mut value = Choice::Players(vec![Player { health: 1 }, Player { health: 2 }]);
        let mut frame = unsafe {
            Frame::new(
                &table,
                vec![Root {
                    ptr: (&raw mut value).cast(),
                    ty: 0,
                    writable: true,
                    receiver: false,
                    born: None,
                }],
            )
        };
        let root = frame.root(0).unwrap();
        let v = frame.field(&root, 0).unwrap();
        let a = node(frame.index(&v, 0).unwrap());
        let b = node(frame.index(&v, 1).unwrap());
        let _again = frame.field(&root, 0).unwrap();
        frame.set(&a, 0, Value::S(7)).unwrap();
        frame.set(&b, 0, Value::S(8)).unwrap();
        assert_eq!(frame.get(&a, 0).unwrap(), Value::S(7));
        assert_eq!(frame.get(&b, 0).unwrap(), Value::S(8));
        frame.apply();
        assert_eq!(frame.get(&a, 0).unwrap(), Value::S(7));
        assert_eq!(frame.get(&b, 0).unwrap(), Value::S(8));
        // A shared enum prefix reborrows, so it is deliberately not provably disjoint.
        assert!(
            frame
                .call(2, None, vec![Call::Node(&a), Call::Node(&b)])
                .is_err()
        );
    }

    #[test]
    fn writes_are_typed_and_bounded_before_journaling() {
        let mut game = world();
        let mut frame = frame(&mut game);
        let root = frame.root(0).unwrap();
        assert!(frame.set(&root, 1, Value::S(1)).is_err());
        assert!(
            frame
                .set(&root, 1, Value::Str("x".repeat(1_048_577)))
                .is_err()
        );
        for _ in 0..4096 {
            frame.set(&root, 1, Value::B(true)).unwrap();
        }
        assert_eq!(
            frame.set(&root, 1, Value::B(true)),
            Err(not_here("internals budget"))
        );
        drop(frame);
        assert!(!game.flag);
    }

    #[test]
    fn a_read_only_root_takes_no_writes() {
        let mut game = world();
        let root = Root {
            ptr: &mut game as *mut Game as *mut (),
            ty: GAME,
            writable: false,
            receiver: true,
            born: None,
        };
        // SAFETY: as in `frame`.
        let mut frame = unsafe { Frame::new(&TABLE, vec![root]) };
        frame.allow_members([(GAME, 0), (GAME, 1)]);
        let root = frame.root(0).unwrap();
        assert!(frame.set(&root, 1, Value::B(true)).is_err());
        let players = frame.field(&root, 0).unwrap();
        let first = node(frame.index(&players, 0).unwrap());
        assert!(frame.set(&first, 0, Value::S(0)).is_err());
        assert!(matches!(
            frame.call(0, Some(&root), Vec::new()),
            Err(Refusal::NotHere(_))
        ));
    }
}
