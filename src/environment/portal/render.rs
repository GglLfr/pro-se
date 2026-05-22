use crate::{
    camera::{CameraPool, CameraPoolQuery, ClipPlane, ClipProjection, PooledCameraSystems, PrimaryCamera},
    environment::portal::{Portal, PortalLink},
    prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (
            force_compute_portal_transform.in_set(PooledCameraSystems::Prepare),
            build_portal_visions.in_set(PooledCameraSystems::Obtain),
        ),
    );
}

pub fn force_compute_portal_transform(mut portals: Query<(&Transform, &mut GlobalTransform), (Without<ChildOf>, Changed<Transform>)>) {
    portals.par_iter_mut().for_each(|(&trns, mut global_trns)| {
        global_trns.set_if_neq(trns.into());
    });
}

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(Transform)]
pub struct PortalVisionViewer;

pub fn build_portal_visions(
    mut commands: Commands,
    mut pool: ResMut<CameraPool>,
    mut pool_query: CameraPoolQuery,
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

        let portal_normal = other_portal_trns.forward();
        let orientation = portal_normal
            .dot(other_portal_trns.translation() - other_camera_local_trns.translation)
            .signum();
        let portal_normal = portal_normal * orientation;
        let d = -portal_normal.dot(other_portal_trns.translation());

        pool.obtain(&mut commands, &mut pool_query, |commands, mut data| {
            *data.projection = Projection::custom(ClipProjection {
                clip: ClipPlane::World(HalfSpace::new(portal_normal.extend(d))),
                ..default()
            });
            commands.entity(data.entity).insert((other_camera_trns, other_camera_local_trns));
        })?;
    }

    Ok(())
}
