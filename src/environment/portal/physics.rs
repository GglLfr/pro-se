use std::cmp::Ordering;

use bevy::ecs::entity::EntityHashMap;

use crate::{
    environment::portal::{Portal, PortalLink},
    prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.register_required_components::<RigidBody, InPortal>()
        .add_systems(Update, (portal_collision_notify, portal_collision_handle).chain());
}

#[derive(Component, Debug, Default)]
pub struct InPortal {
    pub entered: EntityHashMap<bool>,
}

pub fn portal_collision_notify(
    mut collision_starts: MessageReader<CollisionStart>,
    mut query: Query<(Entity, &mut InPortal, Has<Portal>)>,
    transforms: Query<(&Position, &Rotation)>,
) {
    for start in collision_starts.read() {
        match query.get_many_mut([start.collider1, start.collider2]) {
            Err(..) | Ok([(.., true), (.., true)]) | Ok([(.., false), (.., false)]) => continue,
            Ok([(portal, .., true), (entity, mut in_portal, false)]) | Ok([(entity, mut in_portal, false), (portal, .., true)]) => {
                let Ok([(&entity_pos, &_entity_rot), (&portal_pos, &portal_rot)]) = transforms.get_many([entity, portal]) else { continue };

                in_portal.entered.insert(
                    portal,
                    match (*portal_pos - *entity_pos).dot(portal_rot.mul_vec3(Vec3::NEG_Z)).partial_cmp(&0.) {
                        None | Some(Ordering::Equal) => continue,
                        Some(Ordering::Less) => false,
                        Some(Ordering::Greater) => true,
                    },
                );
            }
        }
    }
}

pub fn portal_collision_handle(
    mut in_portals: Query<(&mut InPortal, &mut Position, &mut LinearVelocity), Without<Portal>>,
    portals: Query<(&Position, &Rotation, PortalLink), With<Portal>>,
) {
    in_portals.par_iter_mut().for_each(|(mut in_portal, mut entity_pos, mut entity_vel)| {
        in_portal.entered.retain(|&portal, &mut orientation| {
            let Ok((&portal_pos, &portal_rot, other_portal)) = portals.get(portal) else { return false };
            let Ok((&other_portal_pos, &other_portal_rot, ..)) = portals.get(other_portal.get()) else { return false };

            if matches!(
                (
                    orientation,
                    (*portal_pos - **entity_pos).dot(portal_rot.mul_vec3(Vec3::NEG_Z)).partial_cmp(&0.)
                ),
                (false, Some(Ordering::Greater)) | (true, Some(Ordering::Less))
            ) {
                let map = Affine3A::from_rotation_translation(*other_portal_rot, *other_portal_pos)
                    * Affine3A::from_rotation_translation(*portal_rot, *portal_pos).inverse();

                **entity_pos = map.transform_point3(**entity_pos);
                **entity_vel = map.transform_vector3(**entity_vel);
                false
            } else {
                true
            }
        });
    });
}

#[derive(SystemParam)]
pub struct PortalCollisionHooks<'w, 's> {
    pub has_portal: Query<'w, 's, Has<Portal>>,
}

impl CollisionHooks for PortalCollisionHooks<'_, '_> {}
