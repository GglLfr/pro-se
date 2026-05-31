use bevy::core_pipeline::tonemapping::DebandDither;

use crate::prelude::*;

#[derive(Reflect, Component, ExtractComponent, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(IsDefaultUiCamera, GameCamera, Bloom)]
pub struct PrimaryCamera;

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(
    Camera3d,
    Hdr,
    VolumetricFog {
        ambient_intensity: 0.,
        ..default()
    },
    Msaa::Off,
    DebandDither::Disabled,
)]
pub struct GameCamera;
