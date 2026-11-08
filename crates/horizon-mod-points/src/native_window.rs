//! Checked output for generated native row projections.
//!
//! A reader obtains a path from `Locators`, resolves it afresh against its live root, and
//! emits rows with this builder. The call/value lease must be checked before resolving the
//! path. The builder contains only owned row data and source-local path metadata.

use crate::locators::{Locators, MAX_DEPTH, MAX_LOCATORS, MAX_WINDOW_BYTES, Step};
use mod_api::wire::{Put, Writer};
use mod_api::{Row, RowError, RowWindow};

/// A bounded, all-or-error row window under construction.
pub struct WindowBuilder<'a> {
    locators: &'a mut Locators,
    request: RowWindow,
    rows: Vec<Row>,
    bytes: usize,
}

impl<'a> WindowBuilder<'a> {
    /// Validate the request and obtain its source-local path before reading native data.
    pub fn new(
        locators: &'a mut Locators,
        request: RowWindow,
    ) -> Result<(Self, Vec<Step>), RowError> {
        if request.depth as usize > MAX_DEPTH {
            return Err(RowError::Depth);
        }
        if request.rows as usize > MAX_LOCATORS {
            return Err(RowError::Budget);
        }
        let path = locators.path(request.node)?;
        Ok((
            Self {
                locators,
                request,
                rows: Vec::new(),
                bytes: 0,
            },
            path,
        ))
    }

    /// Requested maximum number of rows, including the requested node itself.
    pub fn remaining(&self) -> usize {
        self.request.rows as usize - self.rows.len()
    }

    /// Add the requested node itself. Its id is already interned in this source.
    pub fn root(
        &mut self,
        parent: Option<u32>,
        field: u32,
        ty: u64,
        value: Option<&dyn Put>,
    ) -> Result<u32, RowError> {
        if !self.rows.is_empty() {
            return Err(RowError::Bounds);
        }
        self.push(self.request.node, parent, field, ty, value)?;
        Ok(self.request.node)
    }

    /// Add an immediate or nested child, assigning a stable id to its path.
    pub fn child(
        &mut self,
        parent: u32,
        step: Step,
        field: u32,
        ty: u64,
        value: Option<&dyn Put>,
    ) -> Result<u32, RowError> {
        if self.remaining() == 0 {
            return Err(RowError::Budget);
        }
        let id = self.locators.intern(Some(parent), step)?;
        self.push(id, Some(parent), field, ty, value)?;
        Ok(id)
    }

    fn push(
        &mut self,
        id: u32,
        parent: Option<u32>,
        field: u32,
        ty: u64,
        value: Option<&dyn Put>,
    ) -> Result<(), RowError> {
        if self.remaining() == 0 {
            return Err(RowError::Budget);
        }
        let mut writer = Writer::new();
        writer.limit(MAX_WINDOW_BYTES - self.bytes);
        if let Some(value) = value {
            writer.put(value);
        }
        if let Some(error) = writer.error() {
            return Err(error.into());
        }
        let bytes = writer.into_bytes();
        self.bytes += bytes.len();
        self.rows.push((id, parent, field, ty, bytes));
        Ok(())
    }

    /// Complete an owned result. On any earlier error the caller discards the builder.
    pub fn finish(self) -> Vec<Row> {
        self.rows
    }
}
