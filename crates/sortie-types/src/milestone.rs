//! Friendly "not in the skeleton yet" errors that name the milestone.
//!
//! Milestones of the project plan:
//! M0 DCS measurements and probe; M1 vertical slice (op_north via FakeDcs and on a live
//! server); M2 generator round trip (Register/Launch/Results); M3 campaign worker loop;
//! M4 developer experience (hot reload, `sortie dev`, record/replay); M5 operations
//! (supervisor, resume from the ring buffer, slot/chat policy).

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Milestone {
    M0,
    M1,
    M2,
    M3,
    M4,
    M5,
}

impl fmt::Display for Milestone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Milestone::M0 => "M0",
            Milestone::M1 => "M1",
            Milestone::M2 => "M2",
            Milestone::M3 => "M3",
            Milestone::M4 => "M4",
            Milestone::M5 => "M5",
        };
        f.write_str(s)
    }
}

/// A feature that is deliberately out of scope for now. Displays as
/// `"<feature> lands in <milestone>"`, e.g. "hot reload lands in M4".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotYet {
    pub feature: &'static str,
    pub milestone: Milestone,
}

impl NotYet {
    #[must_use]
    pub const fn new(feature: &'static str, milestone: Milestone) -> Self {
        Self { feature, milestone }
    }
}

impl fmt::Display for NotYet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} lands in {}", self.feature, self.milestone)
    }
}

impl std::error::Error for NotYet {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message() {
        assert_eq!(
            NotYet::new("hot reload", Milestone::M4).to_string(),
            "hot reload lands in M4"
        );
    }
}
