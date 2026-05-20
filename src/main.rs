pub mod prelude {
    pub use avian3d::prelude::*;
    pub use bevy::{
        camera::{
            CameraUpdateSystems, RenderTarget,
            primitives::Aabb,
            visibility::{RenderLayers, VisibilitySystems},
        },
        ecs::{lifecycle::HookContext, query::QueryData, world::DeferredWorld},
        prelude::*,
        render::{camera::camera_system, view::Hdr},
        window::{PrimaryWindow, WindowCreated, WindowResized, WindowScaleFactorChanged},
    };
    pub use mimalloc_redirect::MiMalloc;
}

use prelude::*;

use crate::environment::portal::{Portal, PortalTo, PortalVisionViewer};

pub mod camera;
pub mod environment;

#[derive(Reflect, States, Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[reflect(State, Debug, Default, Clone, PartialEq, /*TODO 0.19 PartialOrd,*/ Hash)]
pub enum GameState {
    #[default]
    Init,
    Menu,
    Load,
    InGame,
}

#[global_allocator]
static ALLOC: MiMalloc = MiMalloc;

fn report_mimalloc_version(_: &mut App) {
    info!("Using MiMalloc {}.", MiMalloc::get_version());
}

fn main() -> AppExit {
    App::new()
        .add_plugins((
            DefaultPlugins,
            report_mimalloc_version,
            PhysicsPlugins::default(),
            PhysicsDebugPlugin,
            (camera::plugin, environment::plugin),
        ))
        .init_state::<GameState>()
        .add_systems(Startup, game_init)
        .run()
}

fn game_init(
    mut commands: Commands,
    mut next: ResMut<NextState<GameState>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    next.set(GameState::InGame);
    let blocks = [
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [2, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 1, 1, 0, 0, 3],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1],
        [1, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 1, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 1, 1, 1, 0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    ];

    let mut left_portal = default();
    let mut right_portal = default();

    let cube = meshes.add(Cuboid::from_size(Vec3::ONE).mesh());
    let material = materials.add(StandardMaterial::default());

    let start_y = (blocks.len() - 1) as f32 / 2.;
    for (dy, row) in blocks.into_iter().enumerate() {
        let start_x = (row.len() - 1) as f32 / -2.;
        for (dx, block) in row.into_iter().enumerate() {
            let trns = Transform::from_xyz(start_x + dx as f32, start_y - dy as f32, 0.);
            match block {
                0 => {}
                1 => {
                    commands.spawn((trns, Mesh3d(cube.clone()), MeshMaterial3d(material.clone())));
                }
                2 => {
                    left_portal = trns.looking_to(Dir3::X, Dir3::Z);
                }
                3 => {
                    right_portal = trns.looking_to(Dir3::X, Dir3::Z);
                }
                4 => {
                    commands.spawn((trns, PortalVisionViewer));
                }
                unknown => panic!("Unknown block {unknown}"),
            }
        }
    }

    let left_portal = commands.spawn((left_portal, Portal::default())).id();
    commands.spawn((right_portal, Portal::default(), PortalTo(left_portal)));
}
