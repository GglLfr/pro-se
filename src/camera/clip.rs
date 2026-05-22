use bevy::math::AspectRatio;

use crate::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, ClipMaterial>>::default());
}

#[derive(Reflect, Debug, Default, Clone, Copy)]
#[reflect(Debug, Default, Clone)]
pub enum ClipPlane {
    #[default]
    None,
    View(#[reflect(ignore)] HalfSpace),
    World(#[reflect(ignore)] HalfSpace),
}

/// [`PerspectiveProjection`], but uses clip distances instead of oblique projection.
#[derive(Reflect, Debug, Clone)]
#[reflect(Debug, Default, Clone)]
pub struct ClipProjection {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
    pub clip: ClipPlane,
}

impl Default for ClipProjection {
    fn default() -> Self {
        Self {
            fov: PI / 4.,
            near: 0.1,
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
            near: self.near,
            far: self.far,
            near_clip_plane: vec4(0., 0., -1., -self.near),
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
            ClipPlane::View(half_space) => {
                let view_normal = half_space.normal();
                let view_distance = half_space.d();
                let Some(world_normal) = camera_transform.affine().transform_vector3a(view_normal).try_normalize() else { return frustum };

                let view_point = view_normal * -view_distance;
                let world_point = camera_transform.affine().transform_point3a(view_point);
                let world_distance = -world_normal.dot(world_point);

                frustum.half_spaces[Frustum::NEAR_PLANE_IDX] = HalfSpace::new(world_point.extend(world_distance));
            }
            ClipPlane::World(half_space) => frustum.half_spaces[Frustum::NEAR_PLANE_IDX] = half_space,
        }
        frustum
    }
}

#[derive(Reflect, Asset, AsBindGroup, Debug, Default, Clone, Copy)]
pub struct ClipMaterial {}
impl MaterialExtension for ClipMaterial {
    fn vertex_shader() -> ShaderRef {
        //TODO custom vertex shader with clip distance
        ShaderRef::Default
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ExtractedClipPlane {
    //
}
