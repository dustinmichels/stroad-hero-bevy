use std::ops::{Deref, DerefMut};

use bevy::prelude::*;

/// 2D ground position on the simulation plane (x = East/West, y = North/South).
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, Reflect)]
#[reflect(Component)]
pub struct GroundPos(pub Vec2);

impl GroundPos {
    pub const ZERO: Self = Self(Vec2::ZERO);

    pub fn new(x: f32, y: f32) -> Self {
        Self(Vec2::new(x, y))
    }
}

impl From<Vec2> for GroundPos {
    fn from(v: Vec2) -> Self {
        Self(v)
    }
}

impl Deref for GroundPos {
    type Target = Vec2;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GroundPos {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Heading angle in radians, counter-clockwise from +X (East).
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, Reflect)]
#[reflect(Component)]
pub struct Heading(pub f32);

impl Heading {
    pub const ZERO: Self = Self(0.0);

    pub fn new(angle: f32) -> Self {
        Self(angle)
    }

    /// Unit direction vector corresponding to this heading.
    pub fn direction(&self) -> Vec2 {
        Vec2::new(self.0.cos(), self.0.sin())
    }

    /// Creates a heading from a 2D direction vector.
    pub fn from_direction(dir: Vec2) -> Self {
        Self(dir.y.atan2(dir.x))
    }
}

impl From<f32> for Heading {
    fn from(angle: f32) -> Self {
        Self(angle)
    }
}

impl Deref for Heading {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Heading {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
