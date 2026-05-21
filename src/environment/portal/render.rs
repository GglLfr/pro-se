use crate::{
    camera::{CameraPool, CameraPoolQuery, PooledCameraSystems, PrimaryCamera},
    environment::portal::{Portal, PortalLink},
    prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, build_portal_visions.in_set(PooledCameraSystems::Obtain));
}

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(Transform)]
pub struct PortalVisionViewer;

pub fn build_portal_visions(
    mut commands: Commands,
    mut pool: ResMut<CameraPool>,
    mut pool_query: Query<CameraPoolQuery>,
    viewer: Single<&GlobalTransform, With<PortalVisionViewer>>,
    camera: Single<(&GlobalTransform, &Frustum), With<PrimaryCamera>>,
    portals: Query<(&GlobalTransform, &Aabb, &Portal, PortalLink)>,
    transforms: Query<&GlobalTransform>,
) -> Result {
    let (camera_trns, frustum) = camera.into_inner();
    for (portal_trns, portal_aabb, portal, link) in &portals {
        let portal_affine = portal_trns.affine();
        let (scl, ..) = portal_affine.to_scale_rotation_translation();
        if scl.x.abs() < 0.00001 || scl.y.abs() < 0.00001 || scl.z.abs() < 0.00001 {
            continue
        }

        let model_sphere = bevy::camera::primitives::Sphere {
            center: portal_affine.transform_point3a(portal_aabb.center),
            radius: portal_trns.radius_vec3a(portal_aabb.half_extents),
        };

        if !frustum.intersects_sphere(&model_sphere, false) || !frustum.intersects_obb(portal_aabb, &portal_affine, true, false) {
            continue
        }

        let Ok(&other_portal_trns) = transforms.get(link.get()) else { continue };
        let other_camera_trns = other_portal_trns * camera_trns.reparented_to(portal_trns);
        let other_camera_local_trns = Transform::from(other_camera_trns);

        let distance = InfinitePlane3d::new(Dir3::Z).signed_distance(other_portal_trns.to_isometry(), other_camera_local_trns.translation);
        let orientation = distance.signum() * -1.;

        let view_from_world = other_camera_trns.affine().matrix3.inverse();
        let mirror_projection_plane_normal = (view_from_world * (other_portal_trns.back().as_vec3() * orientation)).normalize();
        let mirror_camera_projection = PerspectiveProjection {
            near_clip_plane: mirror_projection_plane_normal
                //TODO this is definitely not right, and sometimes breaks. don't avoid complex math now...
                .extend(view_from_world.mul_vec3(vec3(distance, 0., 0.)).length().copysign(distance) * orientation),
            ..default()
        };

        pool.obtain(&mut commands, &mut pool_query, |commands, mut data| {
            *data.projection = mirror_camera_projection.into();
            commands.entity(data.entity).insert((other_camera_trns, other_camera_local_trns));
        })?;
    }

    Ok(())
}
