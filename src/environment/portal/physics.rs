use std::{cmp::Ordering, ops::Mul};

use avian3d::physics_transform::PhysicsTransformSystems;
use bevy::{
    ecs::entity::{EntityHashMap, EntityHashSet},
    utils::Parallel,
};
use bevy_transform_interpolation::{RotationEasingState, ScaleEasingState, TranslationEasingState};

use crate::{
    environment::portal::{Portal, PortalLink},
    math::AffineExt as _,
    prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.register_required_components::<RigidBody, InPortal>()
        .add_message::<Teleported>()
        .add_systems(
            FixedPostUpdate,
            (portal_collision_notify, portal_collision_handle, portal_collision_cancel)
                .chain()
                .in_set(PhysicsSystems::Writeback)
                .before(PhysicsTransformSystems::PositionToTransform),
        );
}

#[derive(Reflect, Message, EntityEvent, Debug, Clone, Copy)]
#[reflect(Debug, Clone)]
pub struct Teleported {
    #[entity_event]
    pub entity: Entity,
    pub map_transform: Affine3A,
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
                let Entry::Vacant(e) = in_portal.entered.entry(portal) else { continue };
                let Ok([(entity_pos, ..), (portal_pos, portal_rot)]) = transforms.get_many([entity, portal]) else { continue };

                e.insert(
                    match (portal_pos.to_vec3a() - entity_pos.to_vec3a())
                        .dot(portal_rot.mul_vec3a(Vec3A::NEG_Z))
                        .partial_cmp(&0.)
                    {
                        None | Some(Ordering::Equal) => continue,
                        Some(Ordering::Less) => false,
                        Some(Ordering::Greater) => true,
                    },
                );
            }
        }
    }
}

pub fn portal_collision_cancel(mut collision_ends: MessageReader<CollisionEnd>, mut query: Query<(Entity, &mut InPortal, Has<Portal>)>) {
    for end in collision_ends.read() {
        match query.get_many_mut([end.collider1, end.collider2]) {
            Err(..) | Ok([(.., true), (.., true)]) | Ok([(.., false), (.., false)]) => continue,
            Ok([(portal, .., true), (.., mut in_portal, false)]) | Ok([(.., mut in_portal, false), (portal, .., true)]) => {
                in_portal.entered.remove(&portal);
            }
        }
    }
}

pub fn portal_collision_handle(
    mut commands: Commands,
    mut teleported_writer: MessageWriter<Teleported>,
    mut in_portals: Query<
        (
            Entity,
            &mut InPortal,
            &mut Transform,
            &mut Position,
            &mut Rotation,
            &mut LinearVelocity,
            Option<&mut TranslationEasingState>,
            Option<&mut RotationEasingState>,
            Option<&mut ScaleEasingState>,
        ),
        Without<Portal>,
    >,
    portals: Query<(&Position, &Rotation, &GlobalTransform, PortalLink), With<Portal>>,
    mut events: Local<Parallel<Vec<Teleported>>>,
) {
    in_portals.par_iter_mut().for_each_init(
        || events.borrow_local_mut(),
        |events, (entity, mut in_portal, mut entity_trns, mut entity_pos, mut entity_rot, mut entity_vel, translation_state, rotation_state, scale_state)| {
            let mut to_remove = EntityHashSet::new();
            for (&portal, &orientation) in &in_portal.entered {
                let Ok((&portal_pos, &portal_rot, portal_scl, other_portal)) = portals.get(portal) else {
                    to_remove.insert(portal);
                    continue
                };

                let Ok((&other_portal_pos, &other_portal_rot, other_portal_scl, ..)) = portals.get(other_portal.get()) else {
                    to_remove.insert(portal);
                    continue
                };

                let portal_scl = portal_scl.scale();
                let other_portal_scl = other_portal_scl.scale();

                if matches!(
                    (
                        orientation,
                        (portal_pos.to_vec3a() - entity_pos.to_vec3a())
                            .dot(portal_rot.mul_vec3a(Vec3A::NEG_Z))
                            .partial_cmp(&0.)
                    ),
                    (false, Some(Ordering::Greater)) | (true, Some(Ordering::Less))
                ) {
                    let map_transform = Affine3A::from_scale_rotation_translation(other_portal_scl, *other_portal_rot, *other_portal_pos)
                        .mul(Affine3A::from_scale_rotation_translation(portal_scl, *portal_rot, *portal_pos).inverse())
                        .cleanup_z();

                    let entity_affine = map_transform * Affine3A::from_scale_rotation_translation(entity_trns.scale, **entity_rot, **entity_pos)
                        .cleanup_z();

                    let (scl, rot, pos) = entity_affine.to_scale_rotation_translation();

                    **entity_pos = pos;
                    **entity_rot = rot;
                    entity_trns.scale = scl;
                    **entity_vel = map_transform.transform_vector3a(entity_vel.to_vec3a()).to_vec3();

                    if let Some(mut state) = translation_state {
                        state.start = None;
                        state.end = None;
                    }

                    if let Some(mut state) = rotation_state {
                        state.start = None;
                        state.end = None;
                    }

                    if let Some(mut state) = scale_state {
                        state.start = None;
                        state.end = None;
                    }

                    events.push(Teleported { entity, map_transform });
                    in_portal.entered.clear();
                    in_portal.entered.insert(other_portal.get(), !orientation);

                    return
                }
            }

            in_portal.entered.retain(|e, _| !to_remove.contains(e));
        },
    );

    let mut drain = vec![];
    events.drain_into(&mut drain);

    for &e in &drain {
        commands.trigger(e);
    }

    teleported_writer.write_batch(drain);
}

#[derive(SystemParam)]
pub struct PortalCollisionHooks<'w, 's> {
    pub has_portal: Query<'w, 's, Has<Portal>>,
}

impl CollisionHooks for PortalCollisionHooks<'_, '_> {}
