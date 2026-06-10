pub mod prelude {
    pub use std::{cmp::Ordering, f32::consts::PI, mem::replace, ops::Mul, ptr::addr_eq};

    pub use avian3d::{physics_transform::PhysicsTransformSystems, prelude::*};
    #[cfg(feature = "dev")]
    pub use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
    pub use bevy::{
        asset::{AssetHandleProvider, RenderAssetUsages},
        camera::{
            CameraProjection, CameraUpdateSystems, RenderTarget, SubCameraView,
            primitives::{Aabb, Frustum, HalfSpace},
            visibility::{NoAutoAabb, RenderLayers, VisibilitySystems},
        },
        core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transmissive3d, Transparent3d},
        ecs::{
            entity::{EntityHashMap, EntityHashSet},
            lifecycle::HookContext,
            query::{QueryData, QueryItem, ROQueryItem},
            system::{
                ReadOnlySystemParam, SystemParam, SystemParamItem,
                lifetimeless::{Read, Write},
            },
            world::DeferredWorld,
        },
        light::{VolumetricFog, VolumetricLight},
        math::Affine3A,
        mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology},
        pbr::{DrawMaterial, DrawPrepass, ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline, Shadow},
        platform::collections::{HashMap, hash_map::Entry},
        post_process::bloom::Bloom,
        prelude::*,
        render::{
            Extract, Render, RenderApp, RenderPlugin, RenderStartup, RenderSystems,
            camera::camera_system,
            extract_component::{ExtractComponent, ExtractComponentPlugin},
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
        utils::Parallel,
        window::{PrimaryWindow, WindowCreated, WindowResized, WindowScaleFactorChanged},
    };
    pub use bevy_enhanced_input::prelude::{self::*, Cancel, Press, Release};
    pub use bevy_skein::{SkeinAppExt as _, SkeinPlugin};
    pub use bevy_transform_interpolation::{RotationEasingState, ScaleEasingState, TranslationEasingState, prelude::*};
    pub use mimalloc_redirect::MiMalloc;
}

use crate::{
    camera::{Clip, ClipMaterial, DEFAULT_CAMERA_DISTANCE, PrimaryCamera},
    environment::portal::{Portal, PortalCollisionHooks, PortalTeleport, PortalTo, PortalVisionViewer},
    math::TransformExt as _,
    prelude::*,
};

pub mod camera;
pub mod control;
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
            #[cfg(feature = "dev")]
            FpsOverlayPlugin::default(), /* .set(WindowPlugin {
                                             primary_window: Some(Window {
                                                 decorations: false,
                                                 resolution: [720; 2].into(),
                                                 ..default()
                                             }),
                                             ..default()
                                         })*/
            report_mimalloc_version,
            PhysicsPlugins::default().with_collision_hooks::<PortalCollisionHooks>(),
            //PhysicsDebugPlugin,
            EnhancedInputPlugin,
            SkeinPlugin {
                handle_brp: cfg!(feature = "dev"),
            },
            (camera::plugin, control::plugin, environment::plugin, gfx::plugin),
        ))
        .init_state::<GameState>()
        .add_systems(Startup, game_init)
        .add_systems(Update, (move_around, (move_camera, lerp).chain()))
        .add_observer(move_camera_on_portal)
        .run()
}

#[derive(Component, Clone, Copy)]
#[component(on_replace = lerp_on_replace)]
struct Lerp {
    rot: [Quat; 2],
    scl: [Vec3; 2],
    pos: Option<[Vec3; 2]>,
    started: f32,
}

fn move_camera(
    camera: Single<(&mut Transform, Option<&mut Lerp>), (With<PrimaryCamera>, Without<PortalVisionViewer>)>,
    viewer: Single<&Transform, With<PortalVisionViewer>>,
) {
    /*let (mut trns, lerp) = camera.into_inner();
    if let Some(mut lerp) = lerp {
        if let Some(pos) = &mut lerp.pos {
            pos[0] = viewer.translation.with_z(pos[0].z);
            pos[1] = viewer.translation.with_z(DEFAULT_CAMERA_DISTANCE);
        }
    } else {
        trns.rotation = viewer.rotation;
        trns.scale = viewer.scale;
        trns.translation = viewer.translation.with_z(DEFAULT_CAMERA_DISTANCE);
    }*/
}

fn lerp_on_replace(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let &lerp = world.get::<Lerp>(entity).unwrap();
    let mut transform = world.get_mut::<Transform>(entity).unwrap();
    transform.rotation = lerp.rot[1];
    transform.scale = lerp.scl[1];
    if let Some(pos) = lerp.pos {
        transform.translation = pos[1];
    }
}

fn move_camera_on_portal(
    teleported: On<PortalTeleport>,
    mut commands: Commands,
    time: Res<Time>,
    camera: Single<(Entity, &Transform), With<PrimaryCamera>>,
) {
    let new_camera_transform = Transform::from_affine(teleported.map_transform * camera.1.compute_affine());
    commands.entity(camera.0).insert(Lerp {
        rot: [new_camera_transform.rotation, camera.1.rotation],
        scl: [new_camera_transform.scale, camera.1.scale],
        pos: Some([new_camera_transform.translation, camera.1.translation]),
        started: time.elapsed_secs(),
    });

    let (scale, rotation, ..) = teleported.map_transform.to_scale_rotation_translation();
    commands.entity(teleported.entity).insert(Lerp {
        rot: [rotation, Quat::IDENTITY],
        scl: [scale, Vec3::ONE],
        pos: None,
        started: time.elapsed_secs(),
    });
}

fn lerp(mut commands: Commands, time: Res<Time>, mut lerps: Query<(Entity, &mut Transform, &Lerp)>) {
    for (e, mut trns, lerp) in &mut lerps {
        let t = ((time.elapsed_secs() - lerp.started) / 0.32).min(1.);
        let t = t * t * (3. - 2. * t);

        trns.rotation = lerp.rot[0].slerp(lerp.rot[1], t);
        trns.scale = lerp.scl[0].lerp(lerp.scl[1], t);
        if let Some(pos) = lerp.pos {
            trns.translation = pos[0].lerp(pos[1], t);
        }

        if t >= 1. {
            commands.entity(e).remove::<Lerp>();
        }
    }
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
    server: Res<AssetServer>,
    mut next: ResMut<NextState<GameState>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ClipMaterial>>,
) {
    next.set(GameState::InGame);
    commands.spawn((
        SceneRoot(server.load(GltfAssetLabel::Scene(0).from_asset("level.gltf"))),
        ColliderConstructorHierarchy::new(ColliderConstructor::ConvexDecompositionFromMesh),
    ));

    /*let blocks = [
        /*[1, 0, 0, 0, 3, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
        [1, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0],
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],*/
        [1, 9, 9, 9, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        [1, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 9],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9],
        [1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 8, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1],
        [1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1],
        [1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [9, 3, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
        [9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 1],
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 9, 9, 9, 1],
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
                        trns.with_scale(vec3(1., 1., 3.)),
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(material.clone()),
                        RigidBody::Static,
                        Collider::cuboid(1., 1., 1.),
                    ));
                }
                /*2 => {
                    portals[0] = trns
                        .looking_to(Dir3::X, Dir3::Z)
                        .with_translation(trns.translation + Vec3::X * 0.5)
                        .with_scale(vec3(3., 3., 3.))
                }
                3 => {
                    portals[1] = trns
                        .looking_to(Dir3::NEG_Y, Dir3::Z)
                        .with_translation(trns.translation + Vec3::Y * 0.5)
                        .with_scale(vec3(7., 7., 7.))
                }
                4..=7 => {}*/
                2 => {
                    portals[0] = trns
                        .looking_to(Dir3::X, Dir3::Z)
                        .with_translation(trns.translation + Vec3::X * 0.499)
                        .with_scale(vec3(3., 1., 1.))
                }
                3 => {
                    portals[1] = trns
                        .looking_to(Dir3::X, Dir3::Z)
                        .with_translation(trns.translation - Vec3::X * 0.499)
                        .with_scale(vec3(3., 1., 1.))
                }
                4 => {
                    portals[2] = trns
                        .looking_to(Dir3::NEG_Y, Dir3::Z)
                        .with_translation(trns.translation + Vec3::Y * 0.499)
                        .with_scale(vec3(3., 1., 1.))
                }
                5 => {
                    portals[3] = trns
                        .looking_to(Dir3::NEG_Y, Dir3::Z)
                        .with_translation(trns.translation - Vec3::Y * 0.499)
                        .with_scale(vec3(3., 1., 1.))
                }
                6..=7 => {}
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
                        SweptCcd::default(),
                        Collider::capsule(0.4, 0.8),
                        LockedAxes::ROTATION_LOCKED.lock_translation_z(),
                    ));
                }
                9 => {
                    commands.spawn((trns.with_scale(vec3(1., 1., 3.)), Mesh3d(cube.clone()), MeshMaterial3d(material.clone())));
                }
                unknown => panic!("Unknown block {unknown}"),
            }
        }
    }

    commands.insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.4)));
    commands.insert_resource(GlobalAmbientLight { ..default() });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(15., 15.))),
        MeshMaterial3d(materials.add(Clip::from(Color::Srgba(Srgba::RED)))),
        Transform::from_xyz(0., 0., -1.).looking_to(Dir3::NEG_Y, Dir3::Z),
    ));

    commands.spawn((
        SpotLight {
            intensity: 1_000_000.,
            shadows_enabled: true,
            range: 60.,
            inner_angle: std::f32::consts::FRAC_PI_4,
            outer_angle: std::f32::consts::FRAC_PI_3,
            ..default()
        },
        VolumetricLight,
        Transform::from_xyz(6., 6., 8.).looking_at(vec3(-6., -6., -4.), Dir3::Y),
    ));

    //let a = commands.spawn((portals[0], Portal::default())).id();
    //commands.spawn((portals[1], Portal::default(), PortalTo(a)));

    let a = commands.spawn((portals[0], Portal::default())).id();
    //commands.spawn((portals[2], Portal::default(), PortalTo(a)));

    let b = commands.spawn((portals[1], Portal::default())).id();
    //commands.spawn((portals[3], Portal::default(), PortalTo(b)));

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
    commands.entity(b).insert(PortalTo(d));*/
    */
}
