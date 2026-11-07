//! Retained copies feed the same per-view submission route as transient batches.
use super::Game;
use mod_api::__private::game::Fields;
use mod_api::{GroupError, InstanceGroupRef, InstanceGroups, ModelInstance, ModelRef};

const MOD_CAPACITY: usize = 65_536;

struct Group {
    generation: u64,
    owner: Option<&'static str>,
    model: ModelRef,
    copies: Vec<ModelInstance>,
    len: usize,
    last_view: u64,
}

#[derive(Default)]
pub(super) struct InstanceGroupStorage {
    next: u64,
    groups: Vec<Option<Group>>,
}
impl InstanceGroupStorage {
    pub(super) fn clear(&mut self) {
        self.groups.clear();
    }
    fn get(
        &mut self,
        handle: InstanceGroupRef,
        model_generation: u64,
    ) -> Result<&mut Group, GroupError> {
        let fields = handle.fields();
        self.groups
            .get_mut(fields.slot as usize)
            .and_then(Option::as_mut)
            .filter(|g| {
                g.generation == fields.generation && g.model.fields().generation == model_generation
            })
            .ok_or(GroupError::Stale)
    }
}

impl InstanceGroups for Game<'_> {
    fn create_group(&self, model: ModelRef, capacity: u32) -> Result<InstanceGroupRef, GroupError> {
        let fields = model.fields();
        if fields.generation != self.standalone_geometry.generation() {
            return Err(GroupError::Model(mod_api::ModelError::StaleModel));
        }
        let slot = self
            .standalone_geometry
            .instance_slot(fields.slot)
            .ok_or(GroupError::Model(mod_api::ModelError::UnknownModel))?;
        if slot.translucent {
            return Err(GroupError::Translucent);
        }
        let owner = mod_api::crash::current_native_mod();
        let mut storage = self.instance_groups.borrow_mut();
        let reserved: usize = storage
            .groups
            .iter()
            .flatten()
            .filter(|g| g.owner == owner)
            .map(|g| g.copies.len().max(1))
            .sum();
        if capacity > 16_384 || reserved + (capacity as usize).max(1) > MOD_CAPACITY {
            return Err(GroupError::CapacityExceeded);
        }
        self.prepared_model_draws.borrow_mut().reserve(16_384);
        storage.next = storage
            .next
            .checked_add(1)
            .expect("instance group epoch exhausted");
        let group = Group {
            generation: storage.next,
            owner,
            model,
            copies: vec![ModelInstance::at([0.0; 3], 0xFFFF_FFFF); capacity as usize],
            len: 0,
            last_view: 0,
        };
        let index = storage
            .groups
            .iter()
            .position(Option::is_none)
            .unwrap_or(storage.groups.len());
        let handle = mod_api::__private::built::instance_group_ref(group.generation, index as u32);
        if index == storage.groups.len() {
            storage.groups.push(Some(group));
        } else {
            storage.groups[index] = Some(group);
        }
        Ok(handle)
    }
}

impl Game<'_> {
    pub(super) fn instance_group_write(
        &self,
        group: InstanceGroupRef,
        first: u32,
        copies: &[ModelInstance],
    ) -> Result<(), GroupError> {
        let mut storage = self.instance_groups.borrow_mut();
        let group = storage.get(group, self.standalone_geometry.generation())?;
        let end = (first as usize)
            .checked_add(copies.len())
            .filter(|end| *end <= group.copies.len())
            .ok_or(GroupError::OutOfRange)?;
        horizon_client::world_geometry::validate_instances(copies)?;
        group.copies[first as usize..end].copy_from_slice(copies);
        group.len = group.len.max(end);
        Ok(())
    }
    pub(super) fn instance_group_set_len(
        &self,
        group: InstanceGroupRef,
        len: u32,
    ) -> Result<(), GroupError> {
        let mut storage = self.instance_groups.borrow_mut();
        let group = storage.get(group, self.standalone_geometry.generation())?;
        if len as usize > group.copies.len() {
            return Err(GroupError::OutOfRange);
        }
        group.len = len as usize;
        Ok(())
    }
    pub(super) fn instance_group_destroy(
        &self,
        handle: InstanceGroupRef,
    ) -> Result<(), GroupError> {
        let mut storage = self.instance_groups.borrow_mut();
        storage.get(handle, self.standalone_geometry.generation())?;
        storage.groups[handle.fields().slot as usize] = None;
        Ok(())
    }
    pub(super) fn draw_instance_group(
        &self,
        draw: &mut horizon_client::world_geometry::WorldGeometry,
        handle: InstanceGroupRef,
    ) -> Result<(), GroupError> {
        let mut storage = self.instance_groups.borrow_mut();
        let group = storage.get(handle, self.standalone_geometry.generation())?;
        if group.last_view == draw.view_id() {
            return Ok(());
        }
        if !draw.group_room() {
            return Err(GroupError::CapacityExceeded);
        }
        self.draw_validated_model_instances(draw, group.model, &group.copies[..group.len])?;
        group.last_view = draw.view_id();
        draw.mark_group();
        Ok(())
    }
}
