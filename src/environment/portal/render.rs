use crate::{
    camera::{CameraPool, CameraPoolQuery, PooledCameraSystems, PrimaryCamera},
    environment::portal::{Portal, PortalLink},
    prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        PostUpdate,
        build_portal_visions
            .in_set(PooledCameraSystems::Obtain)
            .before(bevy::mesh::mark_3d_meshes_as_changed_if_their_assets_changed),
    );
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
        let world_from_local = portal_trns.affine();
        let model_sphere = bevy::camera::primitives::Sphere {
            center: world_from_local.transform_point3a(portal_aabb.center),
            radius: portal_trns.radius_vec3a(portal_aabb.half_extents),
        };

        if !frustum.intersects_sphere(&model_sphere, false) || !frustum.intersects_obb(portal_aabb, &world_from_local, true, false) {
            continue
        }

        let Ok(&other_portal_trns) = transforms.get(link.get()) else { continue };
        let portal_camera_trns = other_portal_trns * camera_trns.reparented_to(portal_trns);
        let portal_camera_local_trns = Transform::from(portal_camera_trns);

        pool.obtain(&mut commands, &mut pool_query, |commands, data| {
            commands.entity(data.entity).insert((portal_camera_trns, portal_camera_local_trns));
        })?;
    }

    Ok(())
}
