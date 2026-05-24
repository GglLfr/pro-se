pub mod prelude {
    pub use std::{f32::consts::PI, ptr::addr_eq};

    pub use avian3d::prelude::*;
    pub use bevy::{
        asset::{AssetHandleProvider, RenderAssetUsages},
        camera::{
            CameraProjection, CameraUpdateSystems, RenderTarget, SubCameraView,
            primitives::{Aabb, Frustum, HalfSpace},
            visibility::{NoAutoAabb, RenderLayers, VisibilitySystems},
        },
        core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transmissive3d, Transparent3d},
        ecs::{
            lifecycle::HookContext,
            query::{QueryData, QueryItem, ROQueryItem},
            system::{
                ReadOnlySystemParam, SystemParamItem,
                lifetimeless::{Read, Write},
            },
            world::DeferredWorld,
        },
        math::Affine3A,
        mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology},
        pbr::{DrawMaterial, DrawPrepass, ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline, Shadow},
        platform::collections::{HashMap, hash_map::Entry},
        post_process::bloom::Bloom,
        prelude::*,
        render::{
            Extract, Render, RenderApp, RenderPlugin, RenderStartup, RenderSystems,
            camera::camera_system,
            render_phase::{Draw, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderCommandState, TrackedRenderPass},
            render_resource::{
                AsBindGroup, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BufferUsages, DynamicUniformBuffer, PipelineCache,
                RenderPipelineDescriptor, ShaderStages, ShaderType, SpecializedMeshPipelineError, TextureFormat, binding_types::uniform_buffer,
            },
            renderer::{RenderDevice, RenderQueue},
            settings::{RenderCreation, WgpuFeatures, WgpuSettings},
            sync_world::RenderEntity,
            view::{Hdr, ViewTarget},
        },
        shader::{ShaderDefVal, ShaderRef},
        window::{PrimaryWindow, WindowCreated, WindowResized, WindowScaleFactorChanged},
    };
    pub use mimalloc_redirect::MiMalloc;
}

use prelude::*;

use crate::{
    camera::ClipMaterial,
    environment::portal::{Portal, PortalTo, PortalVisionViewer},
};

pub mod camera;
pub mod environment;
pub mod gfx;
pub mod math;

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
            (camera::plugin, environment::plugin, gfx::plugin),
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
            (false, false) => mov.0 - t * 7.5,
            (false, true) => mov.0 - t * 7.5 + 7.5,
            (true, false) => mov.0 + t * 7.5,
            (true, true) => mov.0 + t * 7.5 - 7.5,
        };

        trns.scale.x = match (mov.1, mov.2) {
            (false, false) | (true, false) => t * 15.,
            (false, true) | (true, true) => (1. - t) * 15.,
        };
    }
}

fn game_init(
    mut commands: Commands,
    mut next: ResMut<NextState<GameState>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ClipMaterial>>,
) {
    next.set(GameState::InGame);
    let blocks = [
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        [2, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 5],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 1, 1, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0],
        [0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 1, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 1, 1, 1, 0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [3, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 4],
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    ];

    let mut portals = [Transform::IDENTITY; 6];

    let cube = meshes.add(Cuboid::from_size(Vec3::ONE).mesh());
    let material = materials.add(ClipMaterial::default());

    let start_y = (blocks.len() - 1) as f32 / 2.;
    for (dy, row) in blocks.into_iter().enumerate() {
        let start_x = (row.len() - 1) as f32 / -2.;
        for (dx, block) in row.into_iter().enumerate() {
            let trns = Transform::from_xyz(start_x + dx as f32, start_y - dy as f32, 0.);
            match block {
                0 => {}
                1 => {
                    commands.spawn((
                        trns,
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(material.clone()),
                        RigidBody::Static,
                        Collider::cuboid(1., 1., 1.),
                    ));
                }
                i @ 2..=5 => portals[i - 2] = trns.looking_to(Dir3::X, Dir3::Z),
                i @ 6..=7 => portals[i - 2] = trns.with_scale(vec3(16., 1., 1.)).looking_to(Dir3::Y, Dir3::Z),
                8 => {
                    commands.spawn((trns, PortalVisionViewer));
                }
                unknown => panic!("Unknown block {unknown}"),
            }
        }
    }

    portals[0].translation += vec3(-0.5, 1., 0.);
    portals[1].translation += vec3(-0.5, -1., 0.);
    portals[2].translation += vec3(0.5, -1., 0.);
    portals[3].translation += vec3(0.5, 1., 0.);
    portals[4].translation += vec3(0.5, -1., 0.);
    portals[5].translation += vec3(0.5, 1., 0.);

    let a = commands
        .spawn((portals[0], Portal::default(), Shift(portals[0].translation.y, false, false)))
        .id();
    let c = commands
        .spawn((portals[2], Portal::default(), Shift(portals[2].translation.y, true, false)))
        .id();
    commands.entity(a).insert(PortalTo(c));

    let b = commands
        .spawn((portals[1], Portal::default(), Shift(portals[1].translation.y, false, true)))
        .id();
    let d = commands
        .spawn((portals[3], Portal::default(), Shift(portals[3].translation.y, true, true)))
        .id();
    commands.entity(b).insert(PortalTo(d));

    let e = commands.spawn((portals[4], Portal::default())).id();
    let f = commands.spawn((portals[5], Portal::default())).id();
    commands.entity(e).insert(PortalTo(f));
}
