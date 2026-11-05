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
use std::cell::Cell;

/// Where a root is.
#[derive(Clone, Copy, Debug)]
pub struct Root {
    pub ptr: *mut (),
    /// Its type, an index into the table's types.
    pub ty: u32,
    pub writable: bool,
    /// The game function call that returned it: valid only until the next one. `None` for the
    /// call's own roots, valid for the whole call.
    pub born: Option<u64>,
}

/// A node of the game's state, for one call.
#[derive(Debug)]
pub struct Node {
    pub path: Path<Value>,
    /// Where it last resolved, and in which generation.
    at: Cell<Option<(u64, Resolved)>>,
}

impl Node {
    fn new(path: Path<Value>) -> Self {
        Self {
            path,
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
    /// Bumped by every game function call: every cached pointer is stale after it.
    generation: u64,
}

fn gone(what: &str) -> Refusal {
    Refusal::Gone(what.to_owned())
}

fn not_here(what: &str) -> Refusal {
    Refusal::NotHere(what.to_owned())
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
            generation: 0,
        }
    }

    /// The call's root `n`.
    pub fn root(&self, n: u32) -> Result<Node, Refusal> {
        self.live_root(n)?;
        Ok(Node::new(Path {
            root: n,
            steps: Vec::new(),
        }))
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
        let (_, node) = self
            .table
            .types
            .get(at.ty as usize)
            .ok_or_else(|| gone("no such type"))?;
        // SAFETY: `at` was resolved in this generation from a root by the table's own steps, so it
        // points to a live value of type `at.ty`, writable only when `at.writable`.
        unsafe { node(at.ptr, at.writable, op) }
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
            && generation == self.generation
        {
            return Ok(at);
        }
        let at = self.walk(&node.path)?;
        node.at.set(Some((self.generation, at)));
        Ok(at)
    }

    fn step(&self, node: &Node, step: Step<Value>) -> Result<Reached, Refusal> {
        let at = self.resolve(node)?;
        let reply = match &step {
            Step::Field(field) => self.op(at, Op::Field(*field))?,
            Step::Index(i) => self.op(at, Op::Index(*i))?,
            Step::Key(key) => self.op(at, Op::Key(key))?,
        };
        let mut path = node.path.clone();
        path.steps.push(step);
        Ok(match reply {
            Reply::Node(ptr, ty, writable) => {
                let next = Node::new(path);
                next.at
                    .set(Some((self.generation, Resolved { ptr, ty, writable })));
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
        self.step(node, Step::Key(key))
    }

    /// A copy-typed field of `node`, as this call last wrote it, else as the game has it.
    pub fn get(&self, node: &Node, field: u32) -> Result<Value, Refusal> {
        if let Some(value) = self.journal.get(&node.path, field) {
            return Ok(value.clone());
        }
        match self.op(self.resolve(node)?, Op::Get(field))? {
            Reply::Value(value) => Ok(value),
            _ => Err(not_here("a field that isn't a copy")),
        }
    }

    /// Writes a copy-typed field of `node` when the listener returns, or before its next game
    /// function call.
    pub fn set(&mut self, node: &Node, field: u32, value: Value) -> Result<(), Refusal> {
        if !self.resolve(node)?.writable {
            return Err(not_here("a read-only node"));
        }
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
        // The writes go first, as if the function ran after the listener's code before it; then
        // every node resolves again, since a write may have moved what it points into.
        let refused = self.apply();
        self.generation += 1;
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
        let mut written: Vec<*mut ()> =
            this.filter(|_| thunk.writes & 1 != 0).into_iter().collect();
        let mut lowered = Vec::with_capacity(args.len());
        for (i, (arg, ty)) in args.into_iter().zip(thunk.args).enumerate() {
            let writes = thunk.writes >> (i + 1) & 1 != 0;
            lowered.push(match (arg, ty) {
                (Call::Value(value), None) => Arg::Value(value),
                (Call::Node(node), Some(ty)) => {
                    let ptr = check(node, *ty, writes)?;
                    if writes {
                        written.push(ptr);
                    }
                    Arg::Node(ptr)
                }
                _ => {
                    return Err(Refusal::Changed(
                        "a node where a value goes, or the other way".into(),
                    ));
                }
            });
        }
        // Rust keeps two `&mut` parameters apart; so must the mod's nodes.
        let count = written.len();
        written.sort_unstable();
        written.dedup();
        if written.len() != count {
            return Err(not_here("one place written through two parameters"));
        }
        // SAFETY: each node was resolved in this generation to a live value of the thunk's
        // parameter type, writable where the thunk writes, and no two written ones alias.
        let reply = unsafe { (thunk.call)(this, lowered) }?;
        self.generation += 1;
        let reached = match reply {
            Reply::Node(ptr, ty, writable) => {
                self.roots.push(Root {
                    ptr,
                    ty,
                    writable,
                    born: Some(self.generation),
                });
                Reached::Node(Node::new(Path {
                    root: (self.roots.len() - 1) as u32,
                    steps: Vec::new(),
                }))
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
mod tests {
    use super::*;
    use mod_points::internals::{NodeFn, Thunk};

    /// A tiny game: its players, each with health, and a flag.
    struct Game {
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
            Op::Index(i) => players
                .get_mut(i as usize)
                .map(|p| Reply::Node(p as *mut Player as *mut (), PLAYER, w))
                .ok_or_else(|| not_here("an index past the end")),
            _ => Err(not_here("not on this node")),
        }
    }
    unsafe fn player(p: *mut (), _: bool, op: Op<'_>) -> Result<Reply, Refusal> {
        let player = unsafe { &mut *(p as *mut Player) };
        match op {
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

    static TYPES: [(&str, NodeFn); 3] =
        [("Game", game), ("Vec<Player>", players), ("Player", player)];
    static THUNKS: [Thunk; 3] = [
        Thunk {
            path: "Game::kick",
            receiver: Some(GAME),
            args: &[],
            writes: 1,
            call: kick,
        },
        Thunk {
            path: "Game::first",
            receiver: Some(GAME),
            args: &[],
            writes: 1,
            call: first,
        },
        Thunk {
            path: "heal",
            receiver: None,
            args: &[Some(PLAYER), Some(PLAYER)],
            writes: 0b110,
            call: heal,
        },
    ];
    static TABLE: Table = Table {
        types: &TYPES,
        thunks: &THUNKS,
    };

    fn world() -> Game {
        Game {
            players: (0..3).map(|i| Player { health: 10 + i }).collect(),
            flag: false,
        }
    }

    fn frame(game: &mut Game) -> Frame<'static> {
        let root = Root {
            ptr: game as *mut Game as *mut (),
            ty: GAME,
            writable: true,
            born: None,
        };
        // SAFETY: `game` outlives the frame in every test, and only the frame touches it.
        unsafe { Frame::new(&TABLE, vec![root]) }
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
    fn a_call_refuses_the_wrong_type_and_one_place_written_twice() {
        let mut game = world();
        let mut frame = frame(&mut game);
        let root = frame.root(0).unwrap();
        let players = frame.field(&root, 0).unwrap();
        let a = node(frame.index(&players, 0).unwrap());
        let b = node(frame.index(&players, 1).unwrap());
        let wrong = frame.call(2, None, vec![Call::Node(&a), Call::Node(&players)]);
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
    fn a_read_only_root_takes_no_writes() {
        let mut game = world();
        let root = Root {
            ptr: &mut game as *mut Game as *mut (),
            ty: GAME,
            writable: false,
            born: None,
        };
        // SAFETY: as in `frame`.
        let mut frame = unsafe { Frame::new(&TABLE, vec![root]) };
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
