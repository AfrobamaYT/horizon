//! Call identity selection shared by staging and the launcher's route boundary.

/// A call's resolved catalog identity, not its source spelling.
pub struct CallIdentity<'a> {
    pub call: &'a str,
    pub nth: usize,
    pub ctx: &'a str,
}

/// Selection never silently retargets a moved or ambiguous call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Selected(usize),
    Moved(usize),
    Lost,
    Ambiguous,
}

/// Find the unique context for one callee, then check its per-callee ordinal.
/// Duplicate contexts are ambiguous even when the requested ordinal still matches.
pub fn select(calls: &[CallIdentity<'_>], call: &str, nth: usize, ctx: &str) -> Selection {
    let mut matches = calls
        .iter()
        .enumerate()
        .filter(|(_, c)| c.call == call && c.ctx == ctx);
    let Some((index, found)) = matches.next() else {
        return Selection::Lost;
    };
    if matches.next().is_some() {
        return Selection::Ambiguous;
    }
    if found.nth == nth {
        Selection::Selected(index)
    } else {
        Selection::Moved(found.nth)
    }
}

/// Typed until the source-staging boundary supplies the declaration's location.
#[derive(Debug)]
pub(crate) enum Refusal {
    Moved(usize),
    Lost,
}
impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Moved(nth) => write!(f, "E_INJECT_MOVED: now at nth={nth}; run inject update"),
            Self::Lost => f.write_str("E_INJECT_LOST: call lost or ambiguous; rerun inject add"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_selection_table() {
        for (rows, expected) in [
            (vec![("callee", 1, "ctx")], Selection::Selected(0)),
            (
                vec![("other", 0, "ctx"), ("callee", 1, "ctx")],
                Selection::Selected(1),
            ),
            (
                vec![("callee", 0, "new"), ("callee", 2, "ctx")],
                Selection::Moved(2),
            ),
            (vec![("callee", 1, "changed")], Selection::Lost),
            (vec![("other", 1, "ctx")], Selection::Lost),
            (vec![], Selection::Lost),
            (
                vec![("callee", 1, "ctx"), ("callee", 2, "ctx")],
                Selection::Ambiguous,
            ),
        ] {
            let calls: Vec<_> = rows
                .iter()
                .map(|(call, nth, ctx)| CallIdentity {
                    call,
                    nth: *nth,
                    ctx,
                })
                .collect();
            assert_eq!(select(&calls, "callee", 1, "ctx"), expected, "{rows:?}");
        }
    }
}
