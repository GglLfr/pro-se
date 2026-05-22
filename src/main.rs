pub mod prelude {
    pub use std::f32::consts::PI;

    pub use avian3d::prelude::*;
    pub use bevy::{
        camera::{
            CameraProjection, CameraUpdateSystems, RenderTarget, SubCameraView,
            primitives::{Aabb, Frustum, HalfSpace},
            visibility::{RenderLayers, VisibilitySystems},
        },
        ecs::{lifecycle::HookContext, query::QueryData, world::DeferredWorld},
        pbr::{ExtendedMaterial, MaterialExtension},
        prelude::*,
        render::{
            RenderPlugin,
            camera::camera_system,
            render_resource::AsBindGroup,
            settings::{RenderCreation, WgpuFeatures, WgpuSettings},
            view::Hdr,
        },
        shader::ShaderRef,
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
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    features: WgpuFeatures::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES | WgpuFeatures::CLIP_DISTANCES,
                    ..default()
                }),
                ..default()
            }),
            report_mimalloc_version,
            PhysicsPlugins::default(),
            PhysicsDebugPlugin,
            (camera::plugin, environment::plugin),
        ))
        .init_state::<GameState>()
        .add_systems(Startup, game_init)
        .add_systems(Update, move_around)
        .run()
}

#[derive(Component)]
struct Shift(f32, bool, bool);

fn move_around(time: Res<Time>, mut transforms: Query<(&mut Transform, &Shift)>) {
    let t = (time.elapsed_secs() / 2.).fract();
    for (trns, mov) in &mut transforms {
        let trns = trns.into_inner();
        trns.translation.y = match (mov.1, mov.2) {
            (false, false) => mov.0 - (t * 15. - 1.).max(0.) / 2.,
            (false, true) => mov.0 - ((t * 15. + 1.).min(15.) - 1.) / 2. + 7.,
            (true, false) => mov.0 + (t * 15. - 1.).max(0.) / 2.,
            (true, true) => mov.0 + ((t * 15. + 1.).min(15.) - 1.) / 2. - 7.,
        };

        trns.scale.x = match (mov.1, mov.2) {
            (false, false) | (true, false) => (t * 15. - 1.).max(0.),
            (false, true) | (true, true) => ((1. - t) * 15. - 1.).max(0.),
        };
    }
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
        [0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 1, 1, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0],
        [0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 1, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 1, 1, 1, 0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0],
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    ];

    let mut portals = [Transform::IDENTITY; 4];

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
                i @ 2..=5 => portals[i - 2] = trns.looking_to(Dir3::X, Dir3::Z),
                6 => {
                    commands.spawn((trns, PortalVisionViewer));
                }
                unknown => panic!("Unknown block {unknown}"),
            }
        }
    }

    portals[0].translation += vec3(-0.5, 0.5, 0.);
    portals[1].translation += vec3(-0.5, -0.5, 0.);
    portals[2].translation += vec3(0.5, -0.5, 0.);
    portals[3].translation += vec3(0.5, 0.5, 0.);

    let a = commands
        .spawn((portals[0], Portal::default(), Shift(portals[0].translation.y, false, false)))
        .id();
    let b = commands
        .spawn((portals[1], Portal::default(), Shift(portals[1].translation.y, false, true)))
        .id();

    let c = commands
        .spawn((portals[2], Portal::default(), Shift(portals[2].translation.y, true, false)))
        .id();
    let d = commands
        .spawn((portals[3], Portal::default(), Shift(portals[3].translation.y, true, true)))
        .id();

    commands.entity(a).insert(PortalTo(c));
    commands.entity(b).insert(PortalTo(d));
}
