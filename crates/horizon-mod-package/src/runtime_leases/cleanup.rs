//! Conservative lock-entry collection under the permanent namespace gate.
use super::*;

/// Exclusive namespace ownership, acquired without creating or upgrading a gate.
pub struct Cleanup {
    pub(crate) data: Dir,
    #[cfg(test)]
    directory: Dir,
    _gate: Locked,
}

impl Cleanup {
    pub fn existing(data: &Path) -> Result<Self, Error> {
        let data = open_directory(data)?;
        filesystem::local(&data)?;
        let directory = Dir::from_std_file(retained(
            data.open_dir_nofollow("run-leases")?.into_std_file(),
        )?);
        let gate = open_lock(
            &directory,
            "gate",
            LockMode::Exclusive,
            Release::Close,
            false,
        )?;
        Ok(Self {
            data,
            #[cfg(test)]
            directory,
            _gate: gate,
        })
    }

    /// Live-unit keys are permanent until their unit is removed under its exclusive lease.
    /// Removing keys merely because no process holds the gate would break noncreating entry.
    /// Record collection is separate and retains its bounded trigger.
    pub fn collect(&self) -> Result<usize, Error> {
        Ok(0)
    }
}

#[cfg(test)]
mod tests;
