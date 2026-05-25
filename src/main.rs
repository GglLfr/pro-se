pub mod prelude {
    pub use std::{f32::consts::PI, mem::replace, ptr::addr_eq};

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
                ReadOnlySystemParam, SystemParam, SystemParamItem,
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
    pub use bevy_enhanced_input::prelude::{self::*, Cancel, Press, Release};
    pub use bevy_tnua::prelude::*;
    pub use bevy_tnua_avian3d::prelude::*;
    pub use mimalloc_redirect::MiMalloc;
}

use bevy_tnua::builtins::{TnuaBuiltinJumpConfig, TnuaBuiltinWalkConfig};
use bevy_tnua_avian3d::TnuaAvian3dPlugin;

use crate::{
    camera::ClipMaterial,
    environment::portal::{Portal, PortalCollisionHooks, PortalTo, PortalVisionViewer},
    prelude::*,
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
            PhysicsPlugins::default().with_collision_hooks::<PortalCollisionHooks>(),
            //PhysicsDebugPlugin,
            TnuaControllerPlugin::<ControlScheme>::new(FixedUpdate),
            TnuaAvian3dPlugin::new(FixedUpdate),
            EnhancedInputPlugin,
            (camera::plugin, environment::plugin, gfx::plugin),
        ))
        .init_state::<GameState>()
        .add_systems(Startup, game_init)
        .add_systems(Update, (move_around, apply_controls.in_set(TnuaUserControlsSystems)))
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

#[derive(TnuaScheme)]
#[scheme(basis = TnuaBuiltinWalk)]
enum ControlScheme {
    Jump(TnuaBuiltinJump),
}

fn game_init(
    mut commands: Commands,
    mut next: ResMut<NextState<GameState>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ClipMaterial>>,
    mut control_scheme_configs: ResMut<Assets<ControlSchemeConfig>>,
) {
    next.set(GameState::InGame);
    let blocks = [
        [1, 1, 1, 1, 1, 0, 3, 0, 0, 1, 1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 1],
        [1, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [1, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
        [1, 0, 1, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 1, 1, 1, 0, 1, 1, 0, 0, 1, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        /*[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
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
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],*/
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
                2 => {
                    portals[0] = trns
                        .with_translation(trns.translation + vec3(0.5, 0.5, 0.))
                        .looking_to(Dir3::X, Dir3::Z)
                        .with_scale(Vec3::splat(2.))
                }
                3 => {
                    portals[1] = trns
                        .with_translation(trns.translation + vec3(0.5, 0.5, 0.))
                        .looking_to(Dir3::NEG_Y, Dir3::Z)
                        .with_scale(Vec3::splat(4.))
                }
                4..=7 => {}
                //i @ 2..=5 => portals[i - 2] = trns.looking_to(Dir3::X, Dir3::Z),
                //i @ 6..=7 => portals[i - 2] = trns.with_scale(vec3(16., 1., 1.)).looking_to(Dir3::Y, Dir3::Z),
                8 => {
                    commands.spawn((
                        Mesh3d(meshes.add(Capsule3d {
                            radius: 0.4,
                            half_length: 0.4,
                        })),
                        MeshMaterial3d(material.clone()),
                        PortalVisionViewer,
                        trns,
                        TransformExtrapolation,
                        TransformHermiteEasing,
                        RigidBody::Dynamic,
                        Collider::capsule(0.4, 0.8),
                        TnuaController::<ControlScheme>::default(),
                        TnuaConfig::<ControlScheme>(control_scheme_configs.add(ControlSchemeConfig {
                            basis: TnuaBuiltinWalkConfig {
                                speed: 10.,
                                float_height: 1.,
                                ..default()
                            },
                            jump: TnuaBuiltinJumpConfig { height: 4., ..default() },
                        })),
                        TnuaAvian3dSensorShape(Collider::cylinder(0.39, 0.0)),
                        LockedAxes::ROTATION_LOCKED.lock_translation_z(),
                    ));
                }
                unknown => panic!("Unknown block {unknown}"),
            }
        }
    }

    let a = commands.spawn((portals[0], Portal::default())).id();
    commands.spawn((portals[1], Portal::default(), PortalTo(a)));

    /*portals[0].translation += vec3(-0.5, 1., 0.);
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
    commands.entity(e).insert(PortalTo(f));*/
}

fn apply_controls(keyboard: Res<ButtonInput<KeyCode>>, mut query: Query<&mut TnuaController<ControlScheme>>) {
    let Ok(mut controller) = query.single_mut() else {
        return;
    };
    controller.initiate_action_feeding();

    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::ArrowLeft) {
        direction -= Vec3::X;
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        direction += Vec3::X;
    }

    controller.basis = TnuaBuiltinWalk {
        desired_motion: direction.normalize_or_zero(),
        ..default()
    };

    if keyboard.pressed(KeyCode::Space) {
        controller.action(ControlScheme::Jump(default()));
    }
}
