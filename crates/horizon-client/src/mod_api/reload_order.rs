//! Dependency ordering for named resource reload listeners.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

/// A problem registering or ordering reload listeners.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReloadOrderError {
    /// A prefix is not a namespaced, traversal-free directory path.
    InvalidPrefix { prefix: String, owner: String },
    /// A listener name has already been registered.
    DuplicateListener { id: String, owner: String },
    /// A dependency named a listener that has not been registered.
    UnknownListener { id: String, owner: String },
    /// The listed listeners belong to a dependency cycle.
    Cycle { listeners: Vec<(String, String)> },
}

impl std::fmt::Display for ReloadOrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPrefix { prefix, owner } => {
                write!(f, "mod {owner}: invalid reload prefix {prefix}")
            }
            Self::DuplicateListener { id, owner } => {
                write!(f, "mod {owner}: duplicate reload listener {id}")
            }
            Self::UnknownListener { id, owner } => {
                write!(f, "mod {owner}: unknown reload listener {id}")
            }
            Self::Cycle { listeners } => {
                write!(f, "reload listener dependency cycle")?;
                for (id, owner) in listeners {
                    write!(f, " {id} (mod {owner})")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ReloadOrderError {}

struct Listener<T> {
    id: String,
    owner: String,
    value: T,
}

/// A registry and dependency graph built during reload-listener registration.
///
/// `add_dependency(a, b)` means `a` runs before `b`. Both names must already
/// be registered; this catches mistakes in the registering mod immediately.
pub struct ReloadOrder<T> {
    listeners: Vec<Listener<T>>,
    indices: BTreeMap<String, usize>,
    edges: Vec<(usize, usize)>,
}

impl<T> Default for ReloadOrder<T> {
    fn default() -> Self {
        Self {
            listeners: Vec::new(),
            indices: BTreeMap::new(),
            edges: Vec::new(),
        }
    }
}

impl<T> ReloadOrder<T> {
    /// Register one uniquely named listener.
    pub fn add_listener(
        &mut self,
        id: String,
        owner: String,
        value: T,
    ) -> Result<(), ReloadOrderError> {
        if self.indices.contains_key(&id) {
            return Err(ReloadOrderError::DuplicateListener { id, owner });
        }
        self.indices.insert(id.clone(), self.listeners.len());
        self.listeners.push(Listener { id, owner, value });
        Ok(())
    }

    /// Require `before` to run before `after`.
    pub fn add_dependency(
        &mut self,
        owner: &str,
        before: &str,
        after: &str,
    ) -> Result<(), ReloadOrderError> {
        let index = |id: &str| {
            self.indices
                .get(id)
                .copied()
                .ok_or_else(|| ReloadOrderError::UnknownListener {
                    id: id.to_owned(),
                    owner: owner.to_owned(),
                })
        };
        let edge = (index(before)?, index(after)?);
        if !self.edges.contains(&edge) {
            self.edges.push(edge);
        }
        Ok(())
    }

    /// The registered names in registration order.
    pub fn registry(&self) -> impl Iterator<Item = (&str, &T)> {
        self.listeners
            .iter()
            .map(|listener| (listener.id.as_str(), &listener.value))
    }

    /// The registered dependency edges, as `(before, after)` names.
    pub fn graph(&self) -> impl Iterator<Item = (&str, &str)> {
        self.edges
            .iter()
            .map(|&(a, b)| (self.listeners[a].id.as_str(), self.listeners[b].id.as_str()))
    }

    /// Resolve the graph. An optional vanilla listener gets priority over
    /// unrelated mods, but never over a listener explicitly ordered before it.
    pub(crate) fn sort(self, vanilla: Option<&str>) -> Result<Vec<T>, ReloadOrderError> {
        let mut outgoing = vec![Vec::new(); self.listeners.len()];
        let mut incoming = vec![Vec::new(); self.listeners.len()];
        for &(a, b) in &self.edges {
            outgoing[a].push(b);
            incoming[b].push(a);
        }
        let vanilla = vanilla.and_then(|id| self.indices.get(id).copied());
        let mut before_vanilla = vec![false; self.listeners.len()];
        if let Some(vanilla) = vanilla {
            let mut pending = incoming[vanilla].clone();
            while let Some(node) = pending.pop() {
                if !before_vanilla[node] {
                    before_vanilla[node] = true;
                    pending.extend(incoming[node].iter().copied());
                }
            }
        }
        let priority = |i| match vanilla {
            Some(v) if i == v => 1,
            Some(_) if before_vanilla[i] => 0,
            Some(_) => 2,
            None => 0,
        };
        let mut indegree: Vec<_> = incoming.iter().map(Vec::len).collect();
        let mut ready = BinaryHeap::new();
        for (i, degree) in indegree.iter().enumerate() {
            if *degree == 0 {
                ready.push(Reverse((priority(i), i)));
            }
        }
        let mut order = Vec::with_capacity(self.listeners.len());
        while let Some(Reverse((_, node))) = ready.pop() {
            order.push(node);
            for &next in &outgoing[node] {
                indegree[next] -= 1;
                if indegree[next] == 0 {
                    ready.push(Reverse((priority(next), next)));
                }
            }
        }
        if order.len() != self.listeners.len() {
            // Following remaining incoming edges must eventually revisit a node.
            // It excludes listeners downstream of a cycle from the diagnostic.
            let mut seen = vec![None; self.listeners.len()];
            let mut path = Vec::new();
            let mut node = indegree.iter().position(|&n| n != 0).unwrap();
            loop {
                if let Some(start) = seen[node] {
                    return Err(ReloadOrderError::Cycle {
                        listeners: path[start..]
                            .iter()
                            .map(|&i| {
                                let listener: &Listener<T> = &self.listeners[i];
                                (listener.id.clone(), listener.owner.clone())
                            })
                            .collect(),
                    });
                }
                seen[node] = Some(path.len());
                path.push(node);
                node = *incoming[node].iter().find(|&&i| indegree[i] != 0).unwrap();
            }
        }
        let mut listeners: Vec<_> = self.listeners.into_iter().map(Some).collect();
        Ok(order
            .into_iter()
            .map(|i| listeners[i].take().unwrap().value)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph() -> ReloadOrder<&'static str> {
        let mut graph = ReloadOrder::default();
        for (id, owner) in [
            ("independent", "one"),
            ("vanilla", "minecraft"),
            ("before", "two"),
            ("after", "three"),
        ] {
            graph.add_listener(id.into(), owner.into(), id).unwrap();
        }
        graph
    }

    #[test]
    fn explicit_before_vanilla_does_not_cycle_with_default_order() {
        let mut graph = graph();
        graph.add_dependency("two", "before", "vanilla").unwrap();
        graph.add_dependency("three", "vanilla", "after").unwrap();
        assert_eq!(
            graph.sort(Some("vanilla")).unwrap(),
            ["before", "vanilla", "independent", "after"]
        );
    }

    #[test]
    fn unknown_dependencies_fail_immediately_with_the_owner() {
        let mut graph = graph();
        assert_eq!(
            graph.add_dependency("two", "missing", "before"),
            Err(ReloadOrderError::UnknownListener {
                id: "missing".into(),
                owner: "two".into()
            })
        );
    }

    #[test]
    fn cycle_names_its_mods_but_not_downstream_nodes() {
        let mut graph = graph();
        graph.add_dependency("two", "before", "after").unwrap();
        graph.add_dependency("three", "after", "before").unwrap();
        graph.add_dependency("one", "after", "independent").unwrap();
        let ReloadOrderError::Cycle { listeners } = graph.sort(None).unwrap_err() else {
            panic!("expected cycle")
        };
        assert_eq!(listeners.len(), 2);
        assert!(listeners.contains(&("before".into(), "two".into())));
        assert!(listeners.contains(&("after".into(), "three".into())));
    }
}
