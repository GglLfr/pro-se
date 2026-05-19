use crate::{GameState, prelude::*};

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(ClearColor(Color::NONE))
        .add_systems(OnExit(GameState::Init), spawn_camera);
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        IsDefaultUiCamera,
        Camera3d {
            depth_load_op: default(),
            depth_texture_usages: TextureUsages::RENDER_ATTACHMENT.into(),
        },
        AmbientLight {
            color: Color::WHITE,
            brightness: 0.4,
            affects_lightmapped_meshes: true,
        },
        PointLight {
            range: 40.,
            intensity: 4_000_000.0,
            ..default()
        },
        Transform::from_xyz(0., 0., 30.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
