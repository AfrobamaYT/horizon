use super::*;

impl Mob {
    /// LivingEntity.knockback(power, xd, zd, source) towards the negative
    /// direction. `Creaking.knockback`: a creaking that cannot move is not
    /// shoved. The native knockback hook runs once the knockback resistance
    /// took its share, before the motion changes and the draws of a random
    /// direction.
    pub fn knockback(
        &mut self,
        level: &mut DamageLevel<'_>,
        power: f64,
        xd: f64,
        zd: f64,
        source: &Damage,
    ) {
        self.knockback_by(level, power, xd, zd, source, None);
    }
    /// `LivingEntity.knockback(power, xd, zd, source, damage, comesFromEffect = true)`: an extra
    /// push of a blow (sprint, the Knockback enchantment). A sulfur cube with a block turns it
    /// into its own launch (`SulfurCube.knockback`), scaled by `power * 0.25` and `sqrt(damage)`.
    pub fn knockback_effect(
        &mut self,
        level: &mut DamageLevel<'_>,
        power: f64,
        xd: f64,
        zd: f64,
        source: &Damage,
        damage: f32,
    ) {
        self.knockback_by(level, power, xd, zd, source, Some(damage));
    }
    /// `getAttributeValue(KNOCKBACK_RESISTANCE)`: the mob's own, and what its gear adds.
    pub fn knockback_resistance(&self) -> f64 {
        self.gear_knockback_resistance
            .unwrap_or(self.attrs.knockback_resistance)
    }
    fn knockback_by(
        &mut self,
        level: &mut DamageLevel<'_>,
        power: f64,
        xd: f64,
        zd: f64,
        source: &Damage,
        effect: Option<f32>,
    ) {
        if let Some(damage) = effect
            && self.species() == Species::SulfurCube
            && sulfur_cube::has_body(self)
            && source.attacker.is_some()
            && sulfur_cube::knockback(self, (xd, zd), source.from, damage, Some(power))
        {
            return;
        }
        if creaking::is_creaking(self) && !creaking::can_move(self) {
            return;
        }
        // The native hook runs first, on the raw strength, then the resistance scales what it left.
        let mut knock = crate::health::Knock {
            strength: power,
            x: xd,
            z: zd,
        };
        let pushed = crate::health::Hurting::Mob(self, source);
        if level.knockback_canceled(pushed, &mut knock) {
            return;
        }
        let crate::health::Knock {
            strength: power,
            x: mut xd,
            z: mut zd,
        } = knock;
        let power = power * (1.0 - self.knockback_resistance());
        if power <= 0.0 {
            return;
        }
        self.needs_sync = true;
        while xd * xd + zd * zd < f64::from(1.0e-5_f32) {
            xd = (self.random.next_double() - self.random.next_double()) * 0.01;
            zd = (self.random.next_double() - self.random.next_double()) * 0.01;
        }
        let len = (xd * xd + zd * zd).sqrt();
        let (kx, kz) = (xd / len * power, zd / len * power);
        self.velocity = Vec3::new(
            self.velocity.x / 2.0 - kx,
            if self.on_ground {
                (self.velocity.y / 2.0 + power).min(0.4)
            } else {
                self.velocity.y
            },
            self.velocity.z / 2.0 - kz,
        );
    }
}
