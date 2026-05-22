use crate::prelude::*;

mod clip;
mod def;
mod pool;
pub use clip::*;
pub use def::*;
pub use pool::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((clip::plugin, pool::plugin))
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 0.4,
            affects_lightmapped_meshes: true,
        })
        .add_systems(Startup, spawn_camera);
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        PrimaryCamera,
        RenderLayers::from_layers(&[0, 1]),
        PointLight {
            range: 40.,
            intensity: 4_000_000.0,
            ..default()
        },
        Transform::from_xyz(0., 0., 30.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
