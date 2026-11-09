//! Rigid contact kernel for moving-structure carriers.
//!
//! Slice 5 of the moving structures design
//! (`docs/modding/src/concepts/moving-structures.md`, section "Transform and
//! contact contract"): a pure geometry harness with no `Host`, no `Game` and
//! no gameplay wiring. A caller assembles a [`CarrierGeometry`] from local
//! cells carrying their real [`VoxelShape`]s, then asks for collision-safe
//! translation and rotation steps against static obstacles, or queries the
//! sparse shape with a world-space ray.
//!
//! Contract implemented here:
//! - the world position of local point `p` is `t + c + R(q)(p - c)` with f64
//!   translation and a canonical normalized quaternion;
//! - the broad phase uses the union of transformed cell bounds, and the
//!   narrow phase uses the cells' real voxel boxes as oriented boxes — the
//!   carrier's outer AABB is never a solid shape, so gaps stay traversable;
//! - translation uses a swept separating-axis test and commits only a
//!   collision-safe fraction;
//! - rotation uses conservative advancement over the bounded angular
//!   interval; an interval the iteration budget cannot resolve stops motion
//!   conservatively by refusing the whole step;
//! - simultaneous contacts are reported in stable `(uuid, cell)` order;
//! - box and iteration budgets refuse the step instead of clamping it;
//! - excess motion (more than one block or five degrees per step) is
//!   refused, never clamped, and NaN, infinity, zero quaternions and zero
//!   rotation axes are refused.

use horizon_core::geometry::{Aabb, BlockHit, COLLISION_EPSILON, Direction, Vec3, VoxelShape};
use horizon_core::position::BlockPos;
use std::fmt;

/// Design trust-boundary limit: a translation step moves at most one block
/// per axis. Larger steps are refused, never clamped.
pub const MAX_TRANSLATION: f64 = 1.0;
/// Design trust-boundary limit: a rotation step turns at most five degrees.
/// Larger steps are refused, never clamped.
pub const MAX_ROTATION_RADIANS: f64 = 5.0 * std::f64::consts::PI / 180.0;
/// Conservative advancement consumes at most this share of the measured
/// separation per sub-step, keeping the bound safe and the convergence
/// geometric.
const ADVANCEMENT_SHARE: f64 = 0.5;
/// Remaining step fraction below which a rotation interval counts as swept.
const FRACTION_RESOLUTION: f64 = 1.0e-9;
/// The world coordinate axes, the face axes of axis-aligned obstacle boxes.
const WORLD_AXES: [Vec3; 3] = [
    Vec3::new(1.0, 0.0, 0.0),
    Vec3::new(0.0, 1.0, 0.0),
    Vec3::new(0.0, 0.0, 1.0),
];

/// Why a transform, geometry or step was refused. Refusals never clamp or
/// partially apply: the caller keeps the previous state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// A NaN or infinite translation, pivot, axis, angle or quaternion.
    NonFinite,
    /// A quaternion too small to normalize.
    ZeroQuaternion,
    /// A rotation axis too small to normalize.
    ZeroAxis,
    /// A translation step beyond [`MAX_TRANSLATION`] on an axis.
    TranslationTooFar,
    /// A rotation step beyond [`MAX_ROTATION_RADIANS`].
    RotationTooFar,
    /// Two cells at the same local position.
    DuplicateCells,
    /// More transformed carrier boxes than the budget allows.
    BoxBudget,
    /// More collision iterations than the budget allows.
    IterationBudget,
}
impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::NonFinite => "non-finite motion value",
            Self::ZeroQuaternion => "zero-length quaternion",
            Self::ZeroAxis => "zero-length rotation axis",
            Self::TranslationTooFar => "translation step exceeds one block",
            Self::RotationTooFar => "rotation step exceeds five degrees",
            Self::DuplicateCells => "duplicate local cells",
            Self::BoxBudget => "transformed box budget exhausted",
            Self::IterationBudget => "collision iteration budget exhausted",
        };
        write!(f, "carrier contact refused: {text}")
    }
}
impl std::error::Error for Refused {}

/// Canonical normalized f64 rotation quaternion `(w, x, y, z)`.
///
/// Every constructor normalizes finite nonzero input and canonicalizes the
/// sign — `q` and `-q` encode the same rotation, so the first nonzero
/// component is made nonnegative — making the stored form unique per
/// rotation. NaN, infinity and quaternions too small to normalize are
/// refused. The right-handed convention matches `Vec3::rotate_y`: a positive
/// quarter turn about +Y maps +X to -Z.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rotation {
    w: f64,
    x: f64,
    y: f64,
    z: f64,
}
impl Rotation {
    /// The identity rotation `(1, 0, 0, 0)`.
    pub const IDENTITY: Self = Self {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// Normalizes and canonicalizes finite nonzero components, refusing NaN,
    /// infinity and quaternions too small to normalize.
    pub fn from_components(w: f64, x: f64, y: f64, z: f64) -> Result<Self, Refused> {
        if !w.is_finite() || !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err(Refused::NonFinite);
        }
        // Scale by the largest component before squaring, so subnormal
        // quaternions normalize instead of underflowing to zero.
        let scale = w.abs().max(x.abs().max(y.abs()).max(z.abs()));
        if scale == 0.0 {
            return Err(Refused::ZeroQuaternion);
        }
        let (w, x, y, z) = (w / scale, x / scale, y / scale, z / scale);
        let norm = (w * w + x * x + y * y + z * z).sqrt();
        Ok(Self::canonical(w / norm, x / norm, y / norm, z / norm))
    }

    /// Right-handed rotation about a finite nonzero `axis` by `angle_rad`.
    pub fn from_axis_angle(axis: Vec3, angle_rad: f64) -> Result<Self, Refused> {
        if !axis.is_finite() || !angle_rad.is_finite() {
            return Err(Refused::NonFinite);
        }
        let length = axis.length();
        if length == 0.0 {
            return Err(Refused::ZeroAxis);
        }
        let (s, c) = (angle_rad * 0.5).sin_cos();
        Self::from_components(
            c,
            axis.x / length * s,
            axis.y / length * s,
            axis.z / length * s,
        )
    }

    /// Flips the sign so the first nonzero component is nonnegative.
    fn canonical(w: f64, x: f64, y: f64, z: f64) -> Self {
        let negative =
            w < 0.0 || (w == 0.0 && (x < 0.0 || (x == 0.0 && (y < 0.0 || (y == 0.0 && z < 0.0)))));
        if negative {
            Self {
                w: -w,
                x: -x,
                y: -y,
                z: -z,
            }
        } else {
            Self { w, x, y, z }
        }
    }

    /// The canonical normalized `(w, x, y, z)` components.
    pub fn components(self) -> [f64; 4] {
        [self.w, self.x, self.y, self.z]
    }

    /// Applies the rotation to a vector.
    pub fn rotate(self, p: Vec3) -> Vec3 {
        let qv = Vec3::new(self.x, self.y, self.z);
        let t = qv.cross(p).scale(2.0);
        p.add_vector(t.scale(self.w)).add_vector(qv.cross(t))
    }

    /// Applies the inverse rotation to a vector.
    pub fn inverse_rotate(self, p: Vec3) -> Vec3 {
        let qv = Vec3::new(-self.x, -self.y, -self.z);
        let t = qv.cross(p).scale(2.0);
        p.add_vector(t.scale(self.w)).add_vector(qv.cross(t))
    }

    /// The composition applying `self` first and `next` second, kept
    /// normalized and canonical so repeated composition stays exact.
    pub fn then(self, next: Self) -> Self {
        let w = next.w * self.w - next.x * self.x - next.y * self.y - next.z * self.z;
        let x = next.w * self.x + next.x * self.w + next.y * self.z - next.z * self.y;
        let y = next.w * self.y - next.x * self.z + next.y * self.w + next.z * self.x;
        let z = next.w * self.z + next.x * self.y - next.y * self.x + next.z * self.w;
        // Both inputs are unit quaternions, so the product renormalizes; the
        // identity fallback only guards impossible denormal input.
        Self::from_components(w, x, y, z).unwrap_or(Self::IDENTITY)
    }
}

/// Rigid placement of a carrier: for local point `p`, pivot `c`, translation
/// `t` and rotation `q`, the world position is `t + c + R(q)(p - c)`.
///
/// Translation and pivot are f64; the rotation is canonical and normalized.
/// Rotation steps turn about the world point `translation + pivot`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidTransform {
    translation: Vec3,
    pivot: Vec3,
    rotation: Rotation,
}
impl RigidTransform {
    /// Places the carrier, refusing non-finite translation or pivot.
    pub fn new(translation: Vec3, pivot: Vec3, rotation: Rotation) -> Result<Self, Refused> {
        if !translation.is_finite() || !pivot.is_finite() {
            return Err(Refused::NonFinite);
        }
        Ok(Self {
            translation,
            pivot,
            rotation,
        })
    }

    /// A placement at `translation` with the pivot and rotation at identity.
    pub fn identity_at(translation: Vec3) -> Result<Self, Refused> {
        Self::new(translation, Vec3::ZERO, Rotation::IDENTITY)
    }

    /// The f64 translation `t`.
    pub fn translation(self) -> Vec3 {
        self.translation
    }
    /// The local pivot `c`.
    pub fn pivot(self) -> Vec3 {
        self.pivot
    }
    /// The canonical normalized rotation `q`.
    pub fn rotation(self) -> Rotation {
        self.rotation
    }

    /// `t + c + R(q)(p - c)`.
    pub fn to_world(self, local: Vec3) -> Vec3 {
        self.translation
            .add_vector(self.pivot)
            .add_vector(self.rotation.rotate(local.subtract(self.pivot)))
    }

    /// The inverse of [`Self::to_world`].
    pub fn to_local(self, world: Vec3) -> Vec3 {
        self.pivot.add_vector(
            self.rotation
                .inverse_rotate(world.subtract(self.translation).subtract(self.pivot)),
        )
    }

    /// The placement moved by `delta`, rotation unchanged.
    pub fn translated(self, delta: Vec3) -> Self {
        Self {
            translation: self.translation.add_vector(delta),
            ..self
        }
    }

    /// The placement turned by `angle_rad` about `axis` through the pivot's
    /// world point. Validates its inputs; per-step size limits live in
    /// [`CarrierGeometry::rotate_step`].
    pub fn rotated(self, axis: Vec3, angle_rad: f64) -> Result<Self, Refused> {
        Ok(Self {
            rotation: self
                .rotation
                .then(Rotation::from_axis_angle(axis, angle_rad)?),
            ..self
        })
    }
}

/// One local cell of a carrier: an integer local position plus the block's
/// real collision shape in cell-local coordinates, as block shapes are.
#[derive(Clone, Debug)]
pub struct CarrierCell {
    local: BlockPos,
    shape: VoxelShape,
}
impl CarrierCell {
    /// Assembles the cell. An empty shape is allowed and contributes no
    /// collision boxes.
    pub fn new(local: BlockPos, shape: VoxelShape) -> Self {
        Self { local, shape }
    }
    /// The integer local position.
    pub fn local(&self) -> BlockPos {
        self.local
    }
    /// The real voxel shape in cell-local coordinates.
    pub fn shape(&self) -> &VoxelShape {
        &self.shape
    }
}

/// One contact between a carrier local cell and an obstacle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contact {
    /// The obstacle's UUID; plain world blocks use 0.
    pub uuid: u128,
    /// The obstacle's tie-break cell; for world blocks its block position.
    pub obstacle_cell: BlockPos,
    /// The carrier local cell whose real voxel box met the obstacle.
    pub local_cell: BlockPos,
}
fn contact_order(contact: &Contact) -> (u128, (i32, i32, i32), (i32, i32, i32)) {
    (
        contact.uuid,
        cell_key(contact.obstacle_cell),
        cell_key(contact.local_cell),
    )
}
fn cell_key(position: BlockPos) -> (i32, i32, i32) {
    (position.x, position.y, position.z)
}

/// The collision-safe outcome of one motion step.
#[derive(Clone, Debug, PartialEq)]
pub struct StepContact {
    /// The fraction of the step safe to commit, in `0.0..=1.0`. Translation
    /// fractions scale the step vector; rotation fractions scale the angle.
    pub fraction: f64,
    /// Contacts met during the step, in stable `(uuid, obstacle cell, local
    /// cell)` order so simultaneous ties resolve identically everywhere.
    pub contacts: Vec<Contact>,
}

/// Per-step work budgets (design: budget transformed shape boxes and
/// collision iterations). Building more carrier oriented boxes than
/// `boxes`, or spending more separating-axis pair tests than `iterations`,
/// refuses the step instead of clamping it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContactBudget {
    /// Maximum transformed carrier boxes per configuration.
    pub boxes: usize,
    /// Maximum separating-axis pair tests per step.
    pub iterations: usize,
}
impl Default for ContactBudget {
    /// Room for the design's 4,096-cell carriers of one box each, plus a
    /// generous pool for conservative advancement.
    fn default() -> Self {
        Self {
            boxes: 4096,
            iterations: 65_536,
        }
    }
}

/// A ray hit on the carrier's sparse shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CarrierHit {
    /// The hit point in world space.
    pub location: Vec3,
    /// The hit face in world space.
    pub direction: Direction,
    /// The local cell whose real voxel box was hit.
    pub local_cell: BlockPos,
}

/// A static collision participant: an axis-aligned world shape plus the
/// stable identity used for contact ordering. World blocks use UUID 0 and
/// their block position as `cell`.
#[derive(Clone, Debug)]
pub struct Obstacle {
    uuid: u128,
    cell: BlockPos,
    origin: Vec3,
    shape: VoxelShape,
}
impl Obstacle {
    /// A world block at `pos` with its real collision shape in block-local
    /// coordinates.
    pub fn world_block(pos: BlockPos, shape: VoxelShape) -> Self {
        Self {
            uuid: 0,
            cell: pos,
            origin: Vec3::from_block(pos),
            shape,
        }
    }

    /// An obstacle with an explicit identity for contact ordering, its shape
    /// in local coordinates at a finite world `origin`.
    pub fn with_identity(
        uuid: u128,
        cell: BlockPos,
        origin: Vec3,
        shape: VoxelShape,
    ) -> Result<Self, Refused> {
        if !origin.is_finite() {
            return Err(Refused::NonFinite);
        }
        Ok(Self {
            uuid,
            cell,
            origin,
            shape,
        })
    }

    /// The obstacle's UUID; 0 for plain world blocks.
    pub fn uuid(&self) -> u128 {
        self.uuid
    }
    /// The obstacle's tie-break cell.
    pub fn cell(&self) -> BlockPos {
        self.cell
    }
    /// The world origin of the shape's local frame.
    pub fn origin(&self) -> Vec3 {
        self.origin
    }
    /// The obstacle's real voxel shape.
    pub fn shape(&self) -> &VoxelShape {
        &self.shape
    }

    fn world_bounds(&self) -> Option<Aabb> {
        self.shape.bounds().map(|b| b.move_by(self.origin))
    }
    fn world_boxes(&self) -> impl Iterator<Item = Aabb> + '_ {
        self.shape.boxes().iter().map(|b| b.move_by(self.origin))
    }
}

/// A carrier voxel box transformed to an oriented world box.
#[derive(Clone, Copy, Debug)]
struct OrientedBox {
    center: Vec3,
    axes: [Vec3; 3],
    half: Vec3,
    local_cell: BlockPos,
}
impl OrientedBox {
    fn corners(&self) -> [Vec3; 8] {
        let mut corners = [Vec3::ZERO; 8];
        let mut index = 0;
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    corners[index] = self
                        .center
                        .add_vector(self.axes[0].scale(x * self.half.x))
                        .add_vector(self.axes[1].scale(y * self.half.y))
                        .add_vector(self.axes[2].scale(z * self.half.z));
                    index += 1;
                }
            }
        }
        corners
    }
}

/// Runs `f` on every separating axis between an oriented box and a
/// world-axis-aligned box: the six face axes plus normalized cross products,
/// with near-parallel combinations skipped.
fn for_each_sat_axis(axes: &[Vec3; 3], mut f: impl FnMut(Vec3)) {
    for axis in axes.iter().copied().chain(WORLD_AXES) {
        f(axis);
    }
    for u in axes {
        for w in WORLD_AXES {
            let cross = u.cross(w);
            // Below this the cross product is too small for `Vec3::normalize`
            // and the face axes already cover the separation.
            if cross.length_squared() > 1.0e-8 {
                f(cross.normalize());
            }
        }
    }
}

/// A box's projected half-extent on a unit axis.
fn projected_half(half: Vec3, axes: &[Vec3; 3], axis: Vec3) -> f64 {
    axes[0].dot(axis).abs() * half.x
        + axes[1].dot(axis).abs() * half.y
        + axes[2].dot(axis).abs() * half.z
}

/// Static SAT separation between a carrier oriented box and a static world
/// box: the maximum interval gap over the separating axes. Positive values
/// are a lower bound on the boxes' true distance — exactly what
/// conservative advancement needs — and nonpositive values mean touching or
/// overlapping.
fn static_separation(moving: &OrientedBox, obstacle: Aabb) -> f64 {
    let center_offset = obstacle.center().subtract(moving.center);
    let obstacle_half = obstacle.size().scale(0.5);
    let mut best = f64::NEG_INFINITY;
    for_each_sat_axis(&moving.axes, |axis| {
        let gap = center_offset.dot(axis).abs()
            - projected_half(moving.half, &moving.axes, axis)
            - projected_half(obstacle_half, &WORLD_AXES, axis);
        best = best.max(gap);
    });
    best
}

/// Time-parameterized SAT between a carrier oriented box sweeping `delta`
/// over step fraction `0..=1` and a static world box. Returns the first
/// contact fraction — 0.0 when the pair already touches or overlaps — or
/// `None` when the pair provably stays apart for the whole step.
fn swept_contact_fraction(moving: &OrientedBox, delta: Vec3, obstacle: Aabb) -> Option<f64> {
    let center_offset = obstacle.center().subtract(moving.center);
    let obstacle_half = obstacle.size().scale(0.5);
    let mut enter = f64::NEG_INFINITY;
    let mut exit = f64::INFINITY;
    let mut separated = false;
    for_each_sat_axis(&moving.axes, |axis| {
        if separated {
            return;
        }
        let offset = center_offset.dot(axis);
        let reach = projected_half(moving.half, &moving.axes, axis)
            + projected_half(obstacle_half, &WORLD_AXES, axis);
        let speed = delta.dot(axis);
        if speed.abs() < 1.0e-12 {
            // Overlapping or touching axes impose no time window.
            if offset.abs() - reach > COLLISION_EPSILON {
                separated = true;
            }
            return;
        }
        let (first, second) = if speed > 0.0 {
            ((offset - reach) / speed, (offset + reach) / speed)
        } else {
            ((offset + reach) / speed, (offset - reach) / speed)
        };
        enter = enter.max(first);
        exit = exit.min(second);
    });
    if separated || enter > exit.min(1.0) {
        return None;
    }
    Some(enter.clamp(0.0, 1.0))
}

/// Broad-phase bounds inflation so end-of-step touching reaches the narrow
/// phase.
fn sweep_margin() -> Vec3 {
    Vec3::new(COLLISION_EPSILON, COLLISION_EPSILON, COLLISION_EPSILON)
}

/// The sparse shape index of one carrier: its UUID for stable contact
/// ordering, plus local cells with their real voxel boxes.
#[derive(Clone, Debug)]
pub struct CarrierGeometry {
    uuid: u128,
    cells: Vec<CarrierCell>,
}
impl CarrierGeometry {
    /// Assembles the geometry, sorting cells by local position and refusing
    /// duplicates before any collision work, as the design requires.
    pub fn new(uuid: u128, mut cells: Vec<CarrierCell>) -> Result<Self, Refused> {
        cells.sort_by_key(|cell| cell_key(cell.local));
        if cells.windows(2).any(|pair| pair[0].local == pair[1].local) {
            return Err(Refused::DuplicateCells);
        }
        Ok(Self { uuid, cells })
    }

    /// The carrier's UUID.
    pub fn uuid(&self) -> u128 {
        self.uuid
    }

    /// The local cells in stable sorted order.
    pub fn cells(&self) -> &[CarrierCell] {
        &self.cells
    }

    /// Builds every cell's real voxel boxes as world oriented boxes,
    /// accumulating their union bounds. Refuses when the box budget is
    /// exceeded.
    fn oriented_boxes(
        &self,
        placement: &RigidTransform,
        box_budget: usize,
    ) -> Result<(Vec<OrientedBox>, Option<Aabb>), Refused> {
        let rotation = placement.rotation();
        let axes = [
            rotation.rotate(Vec3::new(1.0, 0.0, 0.0)),
            rotation.rotate(Vec3::new(0.0, 1.0, 0.0)),
            rotation.rotate(Vec3::new(0.0, 0.0, 1.0)),
        ];
        let mut result = Vec::new();
        let mut bounds: Option<Aabb> = None;
        for cell in &self.cells {
            let local_origin = Vec3::from_block(cell.local);
            for box_bounds in cell.shape.boxes() {
                if result.len() >= box_budget {
                    return Err(Refused::BoxBudget);
                }
                let half = box_bounds.size().scale(0.5);
                let center = placement.to_world(local_origin.add_vector(box_bounds.center()));
                // The exact world AABB of the oriented box: its half-diagonal
                // reach along the world axes.
                let [hx, hy, hz] = half.components();
                let mut reach = Vec3::ZERO;
                for (axis, h) in axes.iter().zip([hx, hy, hz]) {
                    reach = reach.add_vector(Vec3::new(
                        axis.x.abs() * h,
                        axis.y.abs() * h,
                        axis.z.abs() * h,
                    ));
                }
                let exact_bounds = Aabb::new(center.subtract(reach), center.add_vector(reach));
                bounds = Some(match bounds {
                    None => exact_bounds,
                    Some(previous) => previous.union_bounds(exact_bounds),
                });
                result.push(OrientedBox {
                    center,
                    axes,
                    half,
                    local_cell: cell.local,
                });
            }
        }
        Ok((result, bounds))
    }

    /// The union of the transformed cell bounds at `placement`: the design's
    /// broad-phase bounds. A query volume only, never a solid shape. `None`
    /// when every cell's shape is empty.
    pub fn world_bounds(&self, placement: &RigidTransform) -> Option<Aabb> {
        self.oriented_boxes(placement, usize::MAX)
            .ok()
            .and_then(|(_, bounds)| bounds)
    }

    /// One bounded translation step: a swept separating-axis test of every
    /// broad-phase candidate's real boxes against the cells' real voxel
    /// boxes as oriented boxes. Returns the collision-safe fraction of
    /// `delta` and the contacts met, in stable order. Excess or non-finite
    /// motion and exhausted budgets refuse the step rather than clamping it.
    pub fn translate_step(
        &self,
        placement: &RigidTransform,
        delta: Vec3,
        obstacles: &[Obstacle],
        budget: &ContactBudget,
    ) -> Result<StepContact, Refused> {
        if !delta.is_finite() {
            return Err(Refused::NonFinite);
        }
        if delta
            .components()
            .into_iter()
            .any(|d| d.abs() > MAX_TRANSLATION)
        {
            return Err(Refused::TranslationTooFar);
        }
        let (boxes, bounds) = self.oriented_boxes(placement, budget.boxes)?;
        let Some(current) = bounds else {
            return Ok(StepContact {
                fraction: 1.0,
                contacts: Vec::new(),
            });
        };
        // Broad phase: the union of the transformed bounds at both ends of
        // the sweep.
        let swept = current
            .union_bounds(current.move_by(delta))
            .inflate(sweep_margin());
        let mut fraction = 1.0f64;
        let mut contacts: Vec<Contact> = Vec::new();
        let mut iterations = 0usize;
        for obstacle in obstacles {
            let Some(obstacle_bounds) = obstacle.world_bounds() else {
                continue;
            };
            if !swept.intersects(obstacle_bounds) {
                continue;
            }
            for obstacle_box in obstacle.world_boxes() {
                for carrier_box in &boxes {
                    iterations += 1;
                    if iterations > budget.iterations {
                        return Err(Refused::IterationBudget);
                    }
                    let Some(impact) = swept_contact_fraction(carrier_box, delta, obstacle_box)
                    else {
                        continue;
                    };
                    if impact < fraction - COLLISION_EPSILON {
                        fraction = impact;
                        contacts.clear();
                    }
                    if impact <= fraction + COLLISION_EPSILON {
                        contacts.push(Contact {
                            uuid: obstacle.uuid(),
                            obstacle_cell: obstacle.cell(),
                            local_cell: carrier_box.local_cell,
                        });
                    }
                }
            }
        }
        contacts.sort_by_key(contact_order);
        Ok(StepContact { fraction, contacts })
    }

    /// One bounded rotation step about a world `axis` through the pivot's
    /// world point: conservative advancement over the angular interval.
    /// Returns the accepted fraction of the angle and the contacts at the
    /// stopping configuration, in stable order. Rotation stops at the last
    /// proven-safe configuration when an obstacle meets the sweep; an
    /// interval the iteration budget cannot resolve refuses the whole step.
    pub fn rotate_step(
        &self,
        placement: &RigidTransform,
        axis: Vec3,
        angle_rad: f64,
        obstacles: &[Obstacle],
        budget: &ContactBudget,
    ) -> Result<StepContact, Refused> {
        if !axis.is_finite() || !angle_rad.is_finite() {
            return Err(Refused::NonFinite);
        }
        if axis.length() == 0.0 {
            return Err(Refused::ZeroAxis);
        }
        if angle_rad.abs() > MAX_ROTATION_RADIANS {
            return Err(Refused::RotationTooFar);
        }
        let (boxes, bounds) = self.oriented_boxes(placement, budget.boxes)?;
        if bounds.is_none() || angle_rad == 0.0 || obstacles.is_empty() || boxes.is_empty() {
            return Ok(StepContact {
                fraction: 1.0,
                contacts: Vec::new(),
            });
        }
        // Broad phase: every swept configuration fits inside the sphere the
        // corners' greatest distance from the rotation axis spans around the
        // pivot, because rotation preserves that distance.
        let unit_axis = axis.scale(1.0 / axis.length());
        let pivot_world = placement.translation().add_vector(placement.pivot());
        let mut max_radius = 0.0f64;
        for carrier_box in &boxes {
            for corner in carrier_box.corners() {
                let offset = corner.subtract(pivot_world);
                let radial = offset.subtract(unit_axis.scale(offset.dot(unit_axis)));
                max_radius = max_radius.max(radial.length());
            }
        }
        let swept = Aabb::of_size(
            pivot_world,
            Vec3::new(2.0 * max_radius, 2.0 * max_radius, 2.0 * max_radius),
        )
        .inflate(sweep_margin());
        let candidates: Vec<&Obstacle> = obstacles
            .iter()
            .filter(|obstacle| {
                obstacle
                    .world_bounds()
                    .is_some_and(|bounds| swept.intersects(bounds))
            })
            .collect();
        if candidates.is_empty() {
            return Ok(StepContact {
                fraction: 1.0,
                contacts: Vec::new(),
            });
        }
        // Conservative advancement: each measured separation is a lower
        // bound on true distance, and no carrier point travels further than
        // `angle * max_radius` per unit step fraction, so consuming half the
        // measured separation per sub-step can never skip a contact.
        let travel = angle_rad.abs() * max_radius;
        let mut advanced = 0.0f64;
        let mut last_clear = 0.0f64;
        let mut iterations = 0usize;
        let mut current = *placement;
        loop {
            let (boxes, _) = self.oriented_boxes(&current, budget.boxes)?;
            let mut separation = f64::INFINITY;
            let mut contacts: Vec<Contact> = Vec::new();
            for obstacle in &candidates {
                for obstacle_box in obstacle.world_boxes() {
                    for carrier_box in &boxes {
                        iterations += 1;
                        if iterations > budget.iterations {
                            return Err(Refused::IterationBudget);
                        }
                        let gap = static_separation(carrier_box, obstacle_box);
                        if gap < separation - COLLISION_EPSILON {
                            separation = gap;
                            contacts.clear();
                        }
                        if gap <= separation + COLLISION_EPSILON {
                            contacts.push(Contact {
                                uuid: obstacle.uuid(),
                                obstacle_cell: obstacle.cell(),
                                local_cell: carrier_box.local_cell,
                            });
                        }
                    }
                }
            }
            if separation <= COLLISION_EPSILON {
                // Touching stops here (zero penetration); an overlap, which
                // only a start pose that already penetrated can produce,
                // rolls back to the last proven-safe configuration.
                let fraction = if separation < -COLLISION_EPSILON {
                    last_clear
                } else {
                    advanced
                };
                contacts.sort_by_key(contact_order);
                return Ok(StepContact { fraction, contacts });
            }
            let step = separation * ADVANCEMENT_SHARE / travel;
            if step >= 1.0 - advanced - FRACTION_RESOLUTION {
                return Ok(StepContact {
                    fraction: 1.0,
                    contacts: Vec::new(),
                });
            }
            advanced += step;
            last_clear = advanced;
            current = placement.rotated(axis, angle_rad * advanced)?;
        }
    }

    /// A world-space ray query against the carrier's sparse shape: the ray
    /// is transformed into the local frame and clipped against every cell's
    /// real voxel shape, so gaps between cells stay traversable. The nearest
    /// hit wins, with earlier cells preferred on exact ties (stable order).
    pub fn clip(&self, placement: &RigidTransform, start: Vec3, end: Vec3) -> Option<CarrierHit> {
        let local_start = placement.to_local(start);
        let local_end = placement.to_local(end);
        let mut best: Option<(f64, BlockHit)> = None;
        for cell in &self.cells {
            let Some(hit) = cell.shape.clip(local_start, local_end, cell.local) else {
                continue;
            };
            let distance = local_start.distance_squared(hit.location);
            if best.is_none_or(|(previous, _)| distance < previous) {
                best = Some((distance, hit));
            }
        }
        best.map(|(_, hit)| CarrierHit {
            location: placement.to_world(hit.location),
            direction: world_direction(placement.rotation(), hit.direction),
            local_cell: hit.block,
        })
    }
}

/// Rotates a hit face's normal into world space and snaps it back to a
/// direction.
fn world_direction(rotation: Rotation, direction: Direction) -> Direction {
    let normal = direction.normal();
    Direction::approximate_nearest(rotation.rotate(Vec3::new(
        normal[0] as f64,
        normal[1] as f64,
        normal[2] as f64,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_core::geometry::Axis;
    use std::f64::consts::{FRAC_PI_2, PI};

    const FIVE_DEGREES: f64 = 5.0 * PI / 180.0;
    const PLUS_Y: Vec3 = Vec3::new(0.0, 1.0, 0.0);
    const EPSILON: f64 = 1.0e-9;

    fn two_full_cells() -> Vec<CarrierCell> {
        vec![
            CarrierCell::new(BlockPos::new(0, 0, 0), VoxelShape::block()),
            CarrierCell::new(BlockPos::new(1, 0, 0), VoxelShape::block()),
        ]
    }

    fn scene_placement() -> RigidTransform {
        RigidTransform::identity_at(Vec3::new(10.0, 64.0, 10.0)).unwrap()
    }

    /// Static probe: does the placement touch or overlap any obstacle? A
    /// zero-length translation step reports exactly that.
    fn touches(
        geometry: &CarrierGeometry,
        placement: &RigidTransform,
        obstacles: &[Obstacle],
    ) -> bool {
        !geometry
            .translate_step(placement, Vec3::ZERO, obstacles, &ContactBudget::default())
            .unwrap()
            .contacts
            .is_empty()
    }

    #[test]
    fn transform_maps_local_points_with_pivot_and_rotation() {
        let placement = scene_placement();
        assert_eq!(
            placement.to_world(Vec3::from_block(BlockPos::new(0, 0, 0))),
            Vec3::new(10.0, 64.0, 10.0)
        );
        assert_eq!(
            placement.to_world(Vec3::from_block(BlockPos::new(1, 0, 0))),
            Vec3::new(11.0, 64.0, 10.0)
        );
        // Round trip through a rotated placement with a nonzero pivot.
        let quarter = RigidTransform::new(
            Vec3::new(5.0, 5.0, 5.0),
            Vec3::new(1.0, 1.0, 1.0),
            Rotation::from_axis_angle(PLUS_Y, FRAC_PI_2).unwrap(),
        )
        .unwrap();
        let point = Vec3::new(2.0, 3.0, 4.0);
        let world = quarter.to_world(point);
        let round = quarter.to_local(world);
        assert!(
            (round.x - point.x).abs() < EPSILON
                && (round.y - point.y).abs() < EPSILON
                && (round.z - point.z).abs() < EPSILON,
            "round trip {round:?}"
        );
        // t + c + R(q)(p - c) with a +90° Y turn mapping +X to -Z:
        // (5,5,5) + (1,1,1) + R((1,2,3)) = (6,6,6) + (3,2,-1).
        assert!(
            (world.x - 9.0).abs() < EPSILON
                && (world.y - 8.0).abs() < EPSILON
                && (world.z - 5.0).abs() < EPSILON,
            "got {world:?}"
        );
        // The pivot's world point stays fixed under rotation about it.
        assert_eq!(
            quarter.to_world(Vec3::new(1.0, 1.0, 1.0)),
            Vec3::new(6.0, 6.0, 6.0)
        );
        // Rotating a placement matches composing the rotation directly.
        let rotated = placement.rotated(PLUS_Y, FRAC_PI_2).unwrap();
        assert_eq!(
            rotated.rotation(),
            Rotation::from_axis_angle(PLUS_Y, FRAC_PI_2).unwrap()
        );
    }

    #[test]
    fn rotation_input_is_normalized_canonical_or_refused() {
        assert_eq!(
            Rotation::from_components(2.0, 0.0, 0.0, 0.0)
                .unwrap()
                .components(),
            [1.0, 0.0, 0.0, 0.0]
        );
        assert_eq!(
            Rotation::from_components(-1.0, 0.0, 0.0, 0.0)
                .unwrap()
                .components(),
            [1.0, 0.0, 0.0, 0.0]
        );
        assert_eq!(
            Rotation::from_components(0.0, -2.0, 0.0, 0.0)
                .unwrap()
                .components(),
            [0.0, 1.0, 0.0, 0.0]
        );
        // A subnormal quaternion still normalizes instead of underflowing.
        assert_eq!(
            Rotation::from_components(1.0e-320, 0.0, 0.0, 0.0)
                .unwrap()
                .components(),
            [1.0, 0.0, 0.0, 0.0]
        );
        // A quarter turn about +Y maps +X to -Z (design: positive 90° Y).
        let turned = Rotation::from_axis_angle(PLUS_Y, FRAC_PI_2).unwrap();
        let mapped = turned.rotate(Vec3::new(1.0, 0.0, 0.0));
        assert!(
            (mapped.x).abs() < EPSILON
                && (mapped.y).abs() < EPSILON
                && (mapped.z + 1.0).abs() < EPSILON
        );
        assert_eq!(
            Rotation::from_components(f64::NAN, 0.0, 0.0, 0.0),
            Err(Refused::NonFinite)
        );
        assert_eq!(
            Rotation::from_components(0.0, f64::INFINITY, 0.0, 0.0),
            Err(Refused::NonFinite)
        );
        assert_eq!(
            Rotation::from_components(0.0, 0.0, 0.0, 0.0),
            Err(Refused::ZeroQuaternion)
        );
        assert_eq!(
            Rotation::from_axis_angle(Vec3::ZERO, 1.0),
            Err(Refused::ZeroAxis)
        );
        assert_eq!(
            Rotation::from_axis_angle(PLUS_Y, f64::NAN),
            Err(Refused::NonFinite)
        );
    }

    #[test]
    fn eighteen_five_degree_steps_match_the_acceptance_scene() {
        let geometry = CarrierGeometry::new(7, two_full_cells()).unwrap();
        let mut placement = scene_placement();
        for _ in 0..18 {
            let step = geometry
                .rotate_step(
                    &placement,
                    PLUS_Y,
                    FIVE_DEGREES,
                    &[],
                    &ContactBudget::default(),
                )
                .unwrap();
            assert_eq!(step.fraction, 1.0);
            assert!(step.contacts.is_empty());
            placement = placement
                .rotated(PLUS_Y, FIVE_DEGREES * step.fraction)
                .unwrap();
        }
        let first = placement.to_world(Vec3::from_block(BlockPos::new(0, 0, 0)));
        let second = placement.to_world(Vec3::from_block(BlockPos::new(1, 0, 0)));
        let expected_first = Vec3::new(10.0, 64.0, 10.0);
        let expected_second = Vec3::new(10.0, 64.0, 9.0);
        assert!(
            (first.x - expected_first.x).abs() < EPSILON
                && (first.y - expected_first.y).abs() < EPSILON
                && (first.z - expected_first.z).abs() < EPSILON,
            "first origin {first:?}"
        );
        assert!(
            (second.x - expected_second.x).abs() < EPSILON
                && (second.y - expected_second.y).abs() < EPSILON
                && (second.z - expected_second.z).abs() < EPSILON,
            "second origin {second:?}"
        );
    }

    #[test]
    fn wall_translation_reaches_thirteen_then_blocks_without_penetration() {
        let geometry = CarrierGeometry::new(7, two_full_cells()).unwrap();
        let wall = [Obstacle::world_block(
            BlockPos::new(13, 64, 10),
            VoxelShape::block(),
        )];
        let mut placement = scene_placement();
        // Initial maximum x is 12 (broad phase = union of transformed cell
        // bounds).
        let bounds = geometry.world_bounds(&placement).unwrap();
        assert_eq!(bounds.max(Axis::X), 12.0);
        // First step: one block towards the wall reaches 13 exactly.
        let first = geometry
            .translate_step(
                &placement,
                Vec3::new(1.0, 0.0, 0.0),
                &wall,
                &ContactBudget::default(),
            )
            .unwrap();
        assert_eq!(first.fraction, 1.0);
        assert_eq!(first.contacts.len(), 1);
        placement = placement.translated(Vec3::new(1.0, 0.0, 0.0).scale(first.fraction));
        assert!((geometry.world_bounds(&placement).unwrap().max(Axis::X) - 13.0).abs() < EPSILON);
        // Second step: blocked with zero penetration.
        let second = geometry
            .translate_step(
                &placement,
                Vec3::new(1.0, 0.0, 0.0),
                &wall,
                &ContactBudget::default(),
            )
            .unwrap();
        assert_eq!(second.fraction, 0.0);
        assert!(!second.contacts.is_empty());
        placement = placement.translated(Vec3::new(1.0, 0.0, 0.0).scale(second.fraction));
        assert!(
            geometry.world_bounds(&placement).unwrap().max(Axis::X) <= 13.0 + EPSILON,
            "no penetration into the wall"
        );
    }

    #[test]
    fn translation_stops_touching_from_half_a_block() {
        let geometry = CarrierGeometry::new(7, two_full_cells()).unwrap();
        let wall = [Obstacle::world_block(
            BlockPos::new(13, 64, 10),
            VoxelShape::block(),
        )];
        // A 0.5 gap: cells at x 10.5..12.5, wall face at 13.
        let placement = RigidTransform::identity_at(Vec3::new(10.5, 64.0, 10.0)).unwrap();
        let step = geometry
            .translate_step(
                &placement,
                Vec3::new(1.0, 0.0, 0.0),
                &wall,
                &ContactBudget::default(),
            )
            .unwrap();
        assert!(
            (step.fraction - 0.5).abs() < EPSILON,
            "fraction {}",
            step.fraction
        );
        assert_eq!(step.contacts.len(), 1);
        let moved = placement.translated(Vec3::new(1.0, 0.0, 0.0).scale(step.fraction));
        assert!((geometry.world_bounds(&moved).unwrap().max(Axis::X) - 13.0).abs() < EPSILON);
    }

    #[test]
    fn sparse_gap_column_is_traversable_for_rays() {
        // A three-cell span holding only the x=0 and x=2 cells: the gap
        // column at local x=1 must stay open for rays.
        let geometry = CarrierGeometry::new(
            7,
            vec![
                CarrierCell::new(BlockPos::new(0, 0, 0), VoxelShape::block()),
                CarrierCell::new(BlockPos::new(2, 0, 0), VoxelShape::block()),
            ],
        )
        .unwrap();
        let placement = scene_placement();
        // The outer bounds span the gap; only the sparse query respects it.
        let bounds = geometry.world_bounds(&placement).unwrap();
        assert_eq!(bounds.min(Axis::X), 10.0);
        assert_eq!(bounds.max(Axis::X), 13.0);
        let through_gap = geometry.clip(
            &placement,
            Vec3::new(11.5, 66.0, 10.5),
            Vec3::new(11.5, 63.5, 10.5),
        );
        assert_eq!(through_gap, None, "ray down local (1.5, 2, 0.5) must miss");
        let control = geometry
            .clip(
                &placement,
                Vec3::new(10.5, 66.0, 10.5),
                Vec3::new(10.5, 63.5, 10.5),
            )
            .expect("ray through the x=0 cell hits");
        assert_eq!(control.local_cell, BlockPos::new(0, 0, 0));
        assert_eq!(control.direction, Direction::Up);
        assert!(
            (control.location.x - 10.5).abs() < EPSILON
                && (control.location.y - 65.0).abs() < EPSILON
                && (control.location.z - 10.5).abs() < EPSILON,
            "hit {control:?}"
        );
    }

    #[test]
    fn mid_sweep_obstacle_stops_rotation_with_clear_endpoint_poses() {
        let geometry = CarrierGeometry::new(7, two_full_cells()).unwrap();
        // A block in the sweep quadrant of cell (1,0,0) that neither the
        // initial (z >= 0) nor the final (x <= 1) pose can reach.
        let obstacle = Obstacle::with_identity(
            0,
            BlockPos::new(1, 0, -1),
            Vec3::ZERO,
            VoxelShape::from_box(Aabb::new(
                Vec3::new(1.05, 0.0, -1.55),
                Vec3::new(1.35, 1.0, -1.25),
            ))
            .unwrap(),
        )
        .unwrap();
        let obstacles = [obstacle];
        let mut placement = RigidTransform::identity_at(Vec3::ZERO).unwrap();
        let start_clear = !touches(&geometry, &placement, &obstacles);
        let final_pose = placement.rotated(PLUS_Y, FRAC_PI_2).unwrap();
        let end_clear = !touches(&geometry, &final_pose, &obstacles);
        assert!(
            start_clear && end_clear,
            "both endpoint poses must be clear"
        );
        let mut stopped = None;
        for step in 0..18 {
            let outcome = geometry
                .rotate_step(
                    &placement,
                    PLUS_Y,
                    FIVE_DEGREES,
                    &obstacles,
                    &ContactBudget::default(),
                )
                .unwrap();
            placement = placement
                .rotated(PLUS_Y, FIVE_DEGREES * outcome.fraction)
                .unwrap();
            if outcome.fraction < 1.0 {
                stopped = Some(step);
                assert!(!outcome.contacts.is_empty());
                assert!(
                    outcome
                        .contacts
                        .iter()
                        .all(|contact| contact.local_cell == BlockPos::new(1, 0, 0)),
                    "only the sweeping cell contacts: {:?}",
                    outcome.contacts
                );
                break;
            }
        }
        let step = stopped.expect("rotation must stop mid-sweep");
        assert!(
            step < 17,
            "contact happens inside the sweep, not at the end"
        );
        // Further rotation stays blocked at the touching configuration.
        for _ in 0..3 {
            let outcome = geometry
                .rotate_step(
                    &placement,
                    PLUS_Y,
                    FIVE_DEGREES,
                    &obstacles,
                    &ContactBudget::default(),
                )
                .unwrap();
            assert_eq!(outcome.fraction, 0.0);
            assert!(!outcome.contacts.is_empty());
        }
        // The carrier never reached the 90° pose.
        let second = placement.to_world(Vec3::from_block(BlockPos::new(1, 0, 0)));
        assert!(
            second.z > -0.9 && second.x > 0.5,
            "stopped mid-sweep at {second:?}"
        );
    }

    #[test]
    fn exhausted_budgets_refuse_instead_of_clamping() {
        let geometry = CarrierGeometry::new(7, two_full_cells()).unwrap();
        let placement = scene_placement();
        let wall = [Obstacle::world_block(
            BlockPos::new(13, 64, 10),
            VoxelShape::block(),
        )];
        // Two transformed boxes against a one-box budget.
        assert_eq!(
            geometry.translate_step(
                &placement,
                Vec3::new(1.0, 0.0, 0.0),
                &wall,
                &ContactBudget {
                    boxes: 1,
                    iterations: 100
                }
            ),
            Err(Refused::BoxBudget)
        );
        // Two narrow-phase pair tests against a one-iteration budget.
        assert_eq!(
            geometry.translate_step(
                &placement,
                Vec3::new(1.0, 0.0, 0.0),
                &wall,
                &ContactBudget {
                    boxes: 64,
                    iterations: 1
                }
            ),
            Err(Refused::IterationBudget)
        );
        // Conservative advancement cannot even probe once: an obstacle the
        // rotation's bounding sphere reaches (a block above the sweep).
        let overhead = [Obstacle::world_block(
            BlockPos::new(12, 66, 10),
            VoxelShape::block(),
        )];
        assert_eq!(
            geometry.rotate_step(
                &placement,
                PLUS_Y,
                FIVE_DEGREES,
                &overhead,
                &ContactBudget {
                    boxes: 64,
                    iterations: 0
                }
            ),
            Err(Refused::IterationBudget)
        );
        // The refusal left the placement untouched.
        assert_eq!(
            geometry.world_bounds(&placement).unwrap().max(Axis::X),
            12.0
        );
    }

    #[test]
    fn invalid_motion_values_are_refused() {
        let geometry = CarrierGeometry::new(7, two_full_cells()).unwrap();
        let placement = scene_placement();
        let budget = ContactBudget::default();
        let obstacles = [Obstacle::world_block(
            BlockPos::new(13, 64, 10),
            VoxelShape::block(),
        )];
        assert_eq!(
            geometry.translate_step(
                &placement,
                Vec3::new(f64::NAN, 0.0, 0.0),
                &obstacles,
                &budget
            ),
            Err(Refused::NonFinite)
        );
        assert_eq!(
            geometry.translate_step(
                &placement,
                Vec3::new(0.0, f64::INFINITY, 0.0),
                &obstacles,
                &budget
            ),
            Err(Refused::NonFinite)
        );
        assert_eq!(
            geometry.translate_step(&placement, Vec3::new(1.5, 0.0, 0.0), &obstacles, &budget),
            Err(Refused::TranslationTooFar)
        );
        assert_eq!(
            geometry.translate_step(&placement, Vec3::new(0.0, -2.0, 0.0), &obstacles, &budget),
            Err(Refused::TranslationTooFar)
        );
        assert_eq!(
            geometry.rotate_step(&placement, PLUS_Y, 10.0 * PI / 180.0, &obstacles, &budget),
            Err(Refused::RotationTooFar)
        );
        assert_eq!(
            geometry.rotate_step(&placement, PLUS_Y, f64::NAN, &obstacles, &budget),
            Err(Refused::NonFinite)
        );
        assert_eq!(
            geometry.rotate_step(&placement, Vec3::ZERO, FIVE_DEGREES, &obstacles, &budget),
            Err(Refused::ZeroAxis)
        );
        assert_eq!(
            RigidTransform::new(
                Vec3::new(f64::NAN, 0.0, 0.0),
                Vec3::ZERO,
                Rotation::IDENTITY
            ),
            Err(Refused::NonFinite)
        );
        // Exactly at the limits is accepted.
        assert!(
            geometry
                .translate_step(&placement, Vec3::new(1.0, -1.0, 1.0), &obstacles, &budget)
                .is_ok()
        );
        assert!(
            geometry
                .rotate_step(&placement, PLUS_Y, FIVE_DEGREES, &obstacles, &budget)
                .is_ok()
        );
    }

    #[test]
    fn simultaneous_contacts_stay_in_uuid_and_cell_order() {
        let geometry = CarrierGeometry::new(7, two_full_cells()).unwrap();
        let placement = scene_placement();
        // Three wall slabs meeting the carrier at the same fraction, with
        // identities that only stable (uuid, cell) ordering sorts.
        let slab = |min_z: f64, max_z: f64| {
            VoxelShape::from_box(Aabb::new(
                Vec3::new(13.0, 64.0, min_z),
                Vec3::new(14.0, 65.0, max_z),
            ))
            .unwrap()
        };
        let obstacles = [
            Obstacle::with_identity(9, BlockPos::new(1, 0, 0), Vec3::ZERO, slab(10.0, 10.4))
                .unwrap(),
            Obstacle::with_identity(0, BlockPos::new(5, 0, 0), Vec3::ZERO, slab(10.4, 10.7))
                .unwrap(),
            Obstacle::with_identity(0, BlockPos::new(0, 0, 0), Vec3::ZERO, slab(10.7, 11.0))
                .unwrap(),
        ];
        let step = geometry
            .translate_step(
                &placement,
                Vec3::new(1.0, 0.0, 0.0),
                &obstacles,
                &ContactBudget::default(),
            )
            .unwrap();
        assert_eq!(step.fraction, 1.0);
        assert_eq!(step.contacts.len(), 3);
        assert_eq!(
            step.contacts,
            vec![
                Contact {
                    uuid: 0,
                    obstacle_cell: BlockPos::new(0, 0, 0),
                    local_cell: BlockPos::new(1, 0, 0)
                },
                Contact {
                    uuid: 0,
                    obstacle_cell: BlockPos::new(5, 0, 0),
                    local_cell: BlockPos::new(1, 0, 0)
                },
                Contact {
                    uuid: 9,
                    obstacle_cell: BlockPos::new(1, 0, 0),
                    local_cell: BlockPos::new(1, 0, 0)
                },
            ]
        );
    }

    #[test]
    fn duplicate_cells_are_refused_and_cells_sorted() {
        assert_eq!(
            CarrierGeometry::new(
                7,
                vec![
                    CarrierCell::new(BlockPos::new(1, 0, 0), VoxelShape::block()),
                    CarrierCell::new(BlockPos::new(0, 0, 0), VoxelShape::block()),
                    CarrierCell::new(BlockPos::new(1, 0, 0), VoxelShape::block()),
                ]
            )
            .unwrap_err(),
            Refused::DuplicateCells
        );
        let geometry = CarrierGeometry::new(
            7,
            vec![
                CarrierCell::new(BlockPos::new(1, 0, 0), VoxelShape::block()),
                CarrierCell::new(BlockPos::new(-2, 0, 5), VoxelShape::block()),
            ],
        )
        .unwrap();
        let locals: Vec<BlockPos> = geometry.cells().iter().map(|cell| cell.local()).collect();
        assert_eq!(
            locals,
            vec![BlockPos::new(-2, 0, 5), BlockPos::new(1, 0, 0)]
        );
        assert_eq!(geometry.uuid(), 7);
    }
}
