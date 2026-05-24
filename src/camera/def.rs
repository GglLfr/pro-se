use crate::prelude::*;

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(IsDefaultUiCamera, GameCamera, Bloom)]
pub struct PrimaryCamera;

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(Camera3d, Hdr)]
pub struct GameCamera;
