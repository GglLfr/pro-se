use crate::{camera::PrimaryCamera, prelude::*};

pub(super) fn plugin(app: &mut App) {}

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(Transform)]
pub struct PortalVisionViewer;

/*use bevy::{
    asset::RenderAssetUsages,
    camera::{
        ImageRenderTarget, RenderTarget,
        visibility::{NoAutoAabb, SetViewVisibility, calculate_bounds},
    },
    core_pipeline::{
        core_3d::{
            CORE_3D_DEPTH_FORMAT,
            graph::{Core3d, Node3d},
        },
        prepass::DepthPrepass,
    },
    ecs::{query::QueryItem, system::lifetimeless::Read},
    image::{ImageCompareFunction, ImageSampler, ImageSamplerDescriptor},
    mesh::mark_3d_meshes_as_changed_if_their_assets_changed,
    render::{
        RenderApp,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_graph::{NodeRunError, RenderGraphContext, RenderGraphExt as _, RenderLabel, ViewNode, ViewNodeRunner},
        render_resource::{
            AsBindGroup, CommandEncoderDescriptor, Extent3d, Origin3d, TexelCopyTextureInfo, TextureAspect, TextureDimension, TextureFormat,
        },
        renderer::RenderContext,
        texture::GpuImage,
        view::ViewDepthTexture,
    },
};

use crate::{
    camera::PrimaryCamera,
    environment::portal::{Portal, PortalLink},
    prelude::*,
};

#[derive(Clone, PartialEq, Eq, Hash, Debug, RenderLabel)]
struct CopyDepthTexturePass;

/// The render node that copies the depth buffer from that of the camera to the
/// [`DemoDepthTexture`].
#[derive(Default)]
struct CopyDepthTextureNode;

impl ViewNode for CopyDepthTextureNode {
    type ViewQuery = (Read<PortalCamera>, Read<ViewDepthTexture>);

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, depth_texture): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let image_assets = world.resource::<RenderAssets<GpuImage>>();
        let Some(dst) = image_assets.get(camera.depth.id()) else {
            return Ok(());
        };

        render_context.add_command_buffer_generation_task(move |render_device| {
            let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("copy depth to demo texture command encoder"),
            });

            // Copy from the view's depth texture to the destination depth
            // texture.
            command_encoder.copy_texture_to_texture(
                TexelCopyTextureInfo {
                    texture: &depth_texture.texture,
                    mip_level: 0,
                    origin: Origin3d::default(),
                    aspect: TextureAspect::DepthOnly,
                },
                TexelCopyTextureInfo {
                    texture: &dst.texture,
                    mip_level: 0,
                    origin: Origin3d::default(),
                    aspect: TextureAspect::DepthOnly,
                },
                dst.size,
            );

            command_encoder.finish()
        });

        Ok(())
    }
}

#[derive(Resource)]
struct ScreenQuad(Handle<Mesh>);
impl FromWorld for ScreenQuad {
    fn from_world(world: &mut World) -> Self {
        Self(world.resource_mut::<Assets<Mesh>>().add(Plane3d::new(Vec3::Z, Vec2::ONE)))
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        ExtractComponentPlugin::<PortalCamera>::default(),
        MaterialPlugin::<PortalVisionMaterial>::default(),
    ))
    .init_asset::<PortalVisionMaterial>()
    .init_resource::<ScreenQuad>()
    .add_systems(
        PostUpdate,
        create_portal_views
            .after(VisibilitySystems::CheckVisibility)
            .before(mark_3d_meshes_as_changed_if_their_assets_changed),
    )
    .add_systems(PreUpdate, despawn_portal_cameras);

    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.add_render_graph_node::<ViewNodeRunner<CopyDepthTextureNode>>(Core3d, CopyDepthTexturePass);
        render_app.add_render_graph_edges(Core3d, (Node3d::EndPrepasses, CopyDepthTexturePass, Node3d::MainOpaquePass));
    }
}

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(Transform)]
pub struct PortalVisionViewer;

//TODO pool this instead in the camera module
#[derive(Component, ExtractComponent, Clone)]
pub struct PortalCamera {
    color: Handle<Image>,
    depth: Handle<Image>,
}

#[derive(Component)]
pub struct PortalVisionQuad;

pub fn despawn_portal_cameras(mut commands: Commands, cameras: Query<Entity, Or<(With<PortalCamera>, With<PortalVisionViewer>)>>) {
    for e in cameras {
        commands.entity(e).despawn();
    }
}

pub fn create_portal_views(
    mut commands: Commands,
    viewer: Single<&GlobalTransform, With<PortalVisionViewer>>,
    camera: Single<(&mut VisibleEntities, &GlobalTransform, &Camera), With<PrimaryCamera>>,
    portal_query: Query<(&Portal, PortalLink, &GlobalTransform)>,
    transforms: Query<&GlobalTransform>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<PortalVisionMaterial>>,
    mut test_materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    quad: Res<ScreenQuad>,
) {
    let (mut visible_entities, camera_trns, camera_info) = camera.into_inner();
    let Some(size) = camera_info.computed.target_info.as_ref().map(|info| info.physical_size) else { return };

    let (Some(min), Some(max)) = (
        camera_info.ndc_to_world(camera_trns, vec3(-1., -1., 1f32.next_down())),
        camera_info.ndc_to_world(camera_trns, vec3(1., 1., 1f32.next_down())),
    ) else {
        return
    };
    let half_size = Aabb::from_min_max(min, max).half_extents;

    let mut to_add = vec![];
    for (&portal, portal_link, portal_trns) in portal_query.iter_many(visible_entities.iter(TypeId::of::<Portal>())) {
        let link = portal_link.get();

        let Ok(&exit_trns) = transforms.get(link) else { continue };
        let camera_trns = exit_trns * camera_trns.reparented_to(portal_trns);

        let color = images.add(Image::new_target_texture(size.x, size.y, TextureFormat::Rgba16Float, None));
        let depth = images.add(Image {
            sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                label: Some("custom depth image sampler".to_owned()),
                compare: Some(ImageCompareFunction::Always),
                ..ImageSamplerDescriptor::default()
            }),
            ..Image::new_uninit(
                Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                CORE_3D_DEPTH_FORMAT,
                RenderAssetUsages::RENDER_WORLD,
            )
        });

        commands.spawn((
            RenderLayers::from_layers(&[0]),
            PortalCamera {
                color: color.clone(),
                depth: depth.clone(),
            },
            Transform::from(camera_trns),
            camera_trns,
            Camera3d::default(),
            Camera { order: -1, ..default() },
            RenderTarget::Image(ImageRenderTarget {
                handle: color.clone(),
                scale_factor: 1.,
            }),
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
            Hdr,
            Msaa::Off,
            DepthPrepass,
        ));

        to_add.push({
            let mut cmd = commands.spawn((
                RenderLayers::from_layers(&[1]),
                PortalVisionQuad,
                Aabb::from_min_max(-Vec3::from(half_size), half_size.into()),
                NoAutoAabb,
                //Transform::from_translation((min + max) / 2.).looking_at(camera_trns.translation(), camera_trns.up()),
                //GlobalTransform::from(Transform::from_translation((min + max) / 2.).looking_at(camera_trns.translation(), camera_trns.up())),
                Mesh3d(quad.0.clone()),
                MeshMaterial3d(Handle::<StandardMaterial>::default()),
                Visibility::Inherited,
                InheritedVisibility::VISIBLE,
                ViewVisibility::HIDDEN,
                /*MeshMaterial3d(materials.add(PortalVisionMaterial {
                    color_texture: color,
                    depth_texture: depth,
                })),*/
            ));
            cmd.entry().and_modify(|mut view: Mut<ViewVisibility>| view.set_visible());
            cmd.id()
        });
    }

    visible_entities.entities.entry(TypeId::of::<Mesh3d>()).or_default().append(&mut to_add);
}

#[derive(Reflect, AsBindGroup, Asset, Clone)]
pub struct PortalVisionMaterial {
    #[texture(1)]
    #[sampler(2)]
    color_texture: Handle<Image>,
    #[texture(3, sample_type = "depth")]
    #[sampler(4, sampler_type = "comparison")]
    depth_texture: Handle<Image>,
}

impl Material for PortalVisionMaterial {
    fn enable_shadows() -> bool {
        false
    }
}

/* #[derive(Reflect, Resource, Debug, Clone)]
#[reflect(Resource, Debug, Clone)]
pub struct PortalVisionMesh(pub Handle<Mesh>);

pub fn init_portal_vision_mesh(mut commands: Commands, server: Res<AssetServer>) {
    let min = vec3(-0.5, -0.5, 0.);
    let max = vec3(0.5, 0.5, 1.);

    // Suppose Y-up right hand, and camera look from +Z to -Z
    let vertices = vec![
        // Front
        [min.x, min.y, max.z],
        [max.x, min.y, max.z],
        [max.x, max.y, max.z],
        [min.x, max.y, max.z],
        // Back
        [min.x, max.y, min.z],
        [max.x, max.y, min.z],
        [max.x, min.y, min.z],
        [min.x, min.y, min.z],
        // Right
        [max.x, min.y, min.z],
        [max.x, max.y, min.z],
        [max.x, max.y, max.z],
        [max.x, min.y, max.z],
        // Left
        [min.x, min.y, max.z],
        [min.x, max.y, max.z],
        [min.x, max.y, min.z],
        [min.x, min.y, min.z],
        // Top
        [max.x, max.y, min.z],
        [min.x, max.y, min.z],
        [min.x, max.y, max.z],
        [max.x, max.y, max.z],
        // Bottom
        [max.x, min.y, max.z],
        [min.x, min.y, max.z],
        [min.x, min.y, min.z],
        [max.x, min.y, min.z],
    ];

    let indices = Indices::U32(vec![
        0, 1, 2, 2, 3, 0, // front
        4, 5, 6, 6, 7, 4, // back
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // leftcamera
        16, 17, 18, 18, 19, 16, // top
        20, 21, 22, 22, 23, 20, // bottom
    ]);

    let mesh = server.add(
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
            .with_inserted_indices(indices),
    );

    commands.insert_resource(PortalVisionMesh(mesh));
}

    */
*/
