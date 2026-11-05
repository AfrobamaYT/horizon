//! Parsed native command input, shared by player and console listeners.

use crate::{ItemStack, PlayerRef};

/// One validated native command argument, resolved against its command source.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum CommandArgumentValue {
    /// A `word` or greedy `text` argument, exactly as the dispatcher parsed it.
    String(String),
    /// A signed `integer` argument.
    Integer(i32),
    /// A finite double-precision `number` argument.
    Number(f64),
    /// A `boolean` argument.
    Boolean(bool),
    /// The single online player selected by a `player` argument.
    Player(PlayerRef),
    /// An `item` argument with its components applied and a count of one.
    Item(ItemStack),
    /// A `block_pos` argument resolved from world, relative or local coordinates.
    /// Resolution does not load chunks or require the block to be loaded.
    BlockPos([i32; 3]),
}

/// An argument's registered name and its validated value.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CommandArgument {
    /// The name from the command's usage, without its kind or brackets.
    pub name: String,
    /// The value produced by parsing and source resolution.
    pub value: CommandArgumentValue,
}

impl CommandArgument {
    /// Pair a registered argument name with its parsed value.
    pub fn new(name: String, value: CommandArgumentValue) -> Self {
        Self { name, value }
    }
}

/// A native command parsed once by the server's dispatcher, never a raw line.
///
/// Missing optional arguments are absent. Player selectors must resolve to one
/// online player before any listener runs; a failed parse or resolution produces
/// command feedback instead. Both player and console hooks receive this same data.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CommandInvocation {
    /// The literal path, including the registered command name and subcommands.
    pub literals: Vec<String>,
    /// Present arguments in parse order; omitted optional arguments have no entry.
    pub arguments: Vec<CommandArgument>,
}

impl CommandInvocation {
    /// Build an invocation from its parsed literal path and resolved arguments.
    pub fn new(literals: Vec<String>, arguments: Vec<CommandArgument>) -> Self {
        Self {
            literals,
            arguments,
        }
    }

    /// The last argument registered under `name`, or `None` when it is absent.
    pub fn argument(&self, name: &str) -> Option<&CommandArgumentValue> {
        self.arguments
            .iter()
            .rev()
            .find(|argument| argument.name == name)
            .map(|argument| &argument.value)
    }
}
