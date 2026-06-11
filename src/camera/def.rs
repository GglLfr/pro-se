use bevy::{self, core_pipeline::tonemapping::DebandDither};

use crate::prelude::*;

#[derive(Reflect, Component, ExtractComponent, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(IsDefaultUiCamera, GameCamera, Bloom, DebandDither::Enabled)]
pub struct PrimaryCamera;

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(
    Camera3d,
    Hdr,
    VolumetricFog {
        step_count: 64,
        ambient_intensity: 0.,
        jitter: 1.,
        ..default()
    },
)]
pub struct GameCamera;
