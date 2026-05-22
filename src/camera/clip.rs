use bevy::{
    core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transmissive3d, Transparent3d},
    pbr::{DrawMaterial, DrawPrepass, Shadow},
};

use crate::prelude::*;

pub type ClipMaterial = ExtendedMaterial<StandardMaterial, Clip>;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(MaterialPlugin::<ClipMaterial>::default())
        .add_systems(Startup, add_default_clip_material);

    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app
            .add_systems(RenderStartup, init_clip_planes)
            .add_systems(ExtractSchedule, extract_clip_plane)
            .add_systems(
                Render,
                (
                    prepare_clip_planes.in_set(RenderSystems::PrepareResources),
                    prepare_clip_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            );

        type DrawClipped<T> = (SetClipPlaneBindGroup<CLIP_BIND_GROUP_INDEX>, T);

        fn overwrite<P: PhaseItem, C: 'static + RenderCommand<P, Param: ReadOnlySystemParam> + Send + Sync>(world: &mut World) {
            let state = RenderCommandState::<P, DrawClipped<C>>::new(world);

            let functions = &mut *world.resource_mut::<DrawFunctions<P>>().into_inner().write();
            let id = functions.id::<C>();
            let ptr = functions.get_mut(id).unwrap() as *mut dyn Draw<P>;

            for draw in &mut functions.draw_functions {
                if addr_eq(ptr, &**draw) {
                    *draw = Box::new(state);
                    return
                }
            }

            panic!("Draw function not found");
        }

        let world = render_app.world_mut();
        overwrite::<Shadow, DrawPrepass>(world);
        overwrite::<Transmissive3d, DrawMaterial>(world);
        overwrite::<Transparent3d, DrawMaterial>(world);
        overwrite::<Opaque3d, DrawMaterial>(world);
        overwrite::<AlphaMask3d, DrawMaterial>(world);
    }
}

pub const CLIP_BIND_GROUP_INDEX: usize = bevy::pbr::MATERIAL_BIND_GROUP_INDEX + 1;

pub fn add_default_clip_material(mut materials: ResMut<Assets<ClipMaterial>>) -> Result {
    materials.insert(AssetId::default(), default())?;
    Ok(())
}

#[derive(Reflect, Debug, Default, Clone, Copy)]
#[reflect(Debug, Default, Clone)]
pub enum ClipPlane {
    #[default]
    None,
    World(#[reflect(ignore)] HalfSpace),
}

/// [`PerspectiveProjection`], but uses clip distances instead of oblique projection.
#[derive(Reflect, Debug, Clone)]
#[reflect(Debug, Default, Clone)]
pub struct ClipProjection {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub far: f32,
    pub clip: ClipPlane,
}

impl Default for ClipProjection {
    fn default() -> Self {
        Self {
            fov: PI / 4.,
            far: 1000.,
            aspect_ratio: 1.,
            clip: default(),
        }
    }
}

impl ClipProjection {
    pub fn perspective(&self) -> PerspectiveProjection {
        PerspectiveProjection {
            fov: self.fov,
            aspect_ratio: self.aspect_ratio,
            near: 0.1,
            far: self.far,
            near_clip_plane: vec4(0., 0., -1., -0.1),
        }
    }
}

impl CameraProjection for ClipProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        self.perspective().get_clip_from_view()
    }

    fn get_clip_from_view_for_sub(&self, sub_view: &SubCameraView) -> Mat4 {
        self.perspective().get_clip_from_view_for_sub(sub_view)
    }

    fn update(&mut self, width: f32, height: f32) {
        use bevy::math::AspectRatio;

        self.aspect_ratio = AspectRatio::try_new(width, height)
            .expect("Failed to update PerspectiveProjection: width and height must be positive, non-zero values")
            .ratio();
    }

    fn far(&self) -> f32 {
        self.far
    }

    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8] {
        self.perspective().get_frustum_corners(z_near, z_far)
    }

    fn compute_frustum(&self, camera_transform: &GlobalTransform) -> Frustum {
        let mut frustum = self.perspective().compute_frustum(camera_transform);
        match self.clip {
            ClipPlane::None => {}
            ClipPlane::World(half_space) => frustum.half_spaces[Frustum::NEAR_PLANE_IDX] = half_space,
        }
        frustum
    }
}

#[derive(Reflect, Asset, AsBindGroup, Debug, Default, Clone, Copy)]
#[bindless(index_table(range(50..51), binding(100)))]
pub struct Clip {}
impl MaterialExtension for Clip {
    fn vertex_shader() -> ShaderRef {
        //TODO custom vertex shader with clip distance
        ShaderRef::Path("shaders/clip.wgsl".into())
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor
            .vertex
            .shader_defs
            .push(ShaderDefVal::UInt("CLIP_BIND_GROUP".into(), CLIP_BIND_GROUP_INDEX as u32));

        if let Some(fragment) = &mut descriptor.fragment {
            fragment
                .shader_defs
                .push(ShaderDefVal::UInt("CLIP_BIND_GROUP".into(), CLIP_BIND_GROUP_INDEX as u32));
        }

        descriptor.layout.insert(
            CLIP_BIND_GROUP_INDEX,
            BindGroupLayoutDescriptor::new("clip_bind_group", &[
                uniform_buffer::<ViewClipPlane>(true).build(0, ShaderStages::VERTEX_FRAGMENT)
            ]),
        );
        Ok(())
    }
}

#[derive(ShaderType, Clone, Copy)]
pub struct ViewClipPlane {
    pub normal: Vec3,
    pub distance: f32,
}

#[derive(Resource)]
pub struct ViewClipPlanes {
    pub buffer: DynamicUniformBuffer<ViewClipPlane>,
}

#[derive(Component, Clone, Copy)]
pub struct ViewClipPlaneOffset {
    pub offset: u32,
}

pub fn init_clip_planes(mut commands: Commands, device: Res<RenderDevice>) {
    let mut buffer = DynamicUniformBuffer::default();
    buffer.set_label(Some("view_clip_planes_buffer"));

    if device.limits().max_storage_buffers_per_shader_stage > 0 {
        buffer.add_usages(BufferUsages::STORAGE);
    }

    commands.insert_resource(ViewClipPlanes { buffer });
}

pub fn extract_clip_plane(
    mut commands: Commands,
    mut planes: ResMut<ViewClipPlanes>,
    views: Extract<Query<(RenderEntity, &Projection, &GlobalTransform), With<Camera>>>,
) {
    let buf = &mut planes.buffer;
    buf.clear();

    for (e, projection, trns) in &views {
        let plane = if let Projection::Custom(proj) = projection
            && let Some(proj) = proj.get::<ClipProjection>()
            && let ClipPlane::World(plane) = proj.clip
        {
            ViewClipPlane {
                normal: plane.normal().to_vec3(),
                distance: plane.d(),
            }
        } else {
            let normal = trns.forward().to_vec3a();
            let point = trns.affine().translation + trns.affine().matrix3.z_axis * -0.1;
            let distance = -normal.dot(point);

            ViewClipPlane {
                normal: normal.to_vec3(),
                distance,
            }
        };

        commands.entity(e).insert(ViewClipPlaneOffset { offset: buf.push(&plane) });
    }
}

pub fn prepare_clip_planes(mut planes: ResMut<ViewClipPlanes>, device: Res<RenderDevice>, queue: Res<RenderQueue>) {
    planes.buffer.write_buffer(&device, &queue);
}

#[derive(Component)]
pub struct ViewClipBindGroup {
    pub bind_group: BindGroup,
}

pub fn prepare_clip_bind_groups(
    mut commands: Commands,
    planes: Res<ViewClipPlanes>,
    device: Res<RenderDevice>,
    cache: Res<PipelineCache>,
    views: Query<(Entity,)>,
) {
    let Some(resource) = planes.buffer.binding() else { return };
    for (e,) in &views {
        commands.entity(e).insert(ViewClipBindGroup {
            bind_group: device.create_bind_group(
                "view_clip_plane_bind_group",
                &cache.get_bind_group_layout(&BindGroupLayoutDescriptor::new("clip_bind_group", &[uniform_buffer::<ViewClipPlane>(
                    true,
                )
                .build(0, ShaderStages::VERTEX_FRAGMENT)])),
                &[BindGroupEntry {
                    binding: 0,
                    resource: resource.clone(),
                }],
            ),
        });
    }
}

pub struct SetClipPlaneBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetClipPlaneBindGroup<I> {
    type Param = ();
    type ViewQuery = (Read<ViewClipPlaneOffset>, Read<ViewClipBindGroup>);
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (offset, bind_group): ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<()>,
        _: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &bind_group.bind_group, &[offset.offset]);
        RenderCommandResult::Success
    }
}
