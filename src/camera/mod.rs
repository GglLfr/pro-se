use crate::{gfx::LAYER_PORTAL_RESERVE, prelude::*};

mod clip;
mod def;
mod pool;
pub use clip::*;
pub use def::*;
pub use pool::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((ExtractComponentPlugin::<PrimaryCamera>::default(), clip::plugin, pool::plugin))
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(GlobalAmbientLight::NONE)
        .add_systems(Startup, spawn_camera);

    #[cfg(feature = "dev")]
    {
        use bevy::camera_controller::pan_camera::{PanCamera, PanCameraPlugin};

        app.add_plugins(PanCameraPlugin)
            .register_required_components_with::<PrimaryCamera, PanCamera>(|| PanCamera { pan_speed: 1., ..default() });
    }
}

pub const DEFAULT_CAMERA_DISTANCE: f32 = 20.;

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        PrimaryCamera,
        RenderLayers::from_iter([0, LAYER_PORTAL_RESERVE]),
        Transform::from_xyz(0., 0., DEFAULT_CAMERA_DISTANCE).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
