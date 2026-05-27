use crate::{
    camera::ClipMaterial,
    environment::portal::{PortalConnections, PortalLink},
    math::GlobalTransformExt as _,
    prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.register_required_components::<PortalConnections, PortalConnectionDuplicates>()
        .add_systems(
            PostUpdate,
            create_portal_duplicates::<ClipMaterial>
                .after(TransformSystems::Propagate)
                .before(VisibilitySystems::CalculateBounds),
        );
}

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct PortalConnectionDuplicates {
    pub duplicates: EntityHashMap<Entity>,
}

pub fn create_portal_duplicates<T: Material>(
    mut commands: Commands,
    mut meshes: Query<
        (
            Entity,
            &Mesh3d,
            &MeshMaterial3d<T>,
            &Visibility,
            &PortalConnections,
            &mut PortalConnectionDuplicates,
        ),
        Changed<PortalConnections>,
    >,
    mut transforms: Query<(&mut Transform, &mut GlobalTransform)>,
    portals: Query<PortalLink>,
) {
    for (entity, mesh, material, &visibility, connection, mut duplicates) in &mut meshes {
        duplicates.retain(|portal, &mut dupe| match connection.contains_key(portal) {
            false => {
                commands.entity(dupe).despawn();
                false
            }
            true => true,
        });

        for &portal in connection.keys() {
            let Ok([(.., &entity_trns), (.., &portal_trns), (.., &other_portal_trns)]) =
                portals.get(portal).and_then(|link| transforms.get_many([entity, portal, link.get()]))
            else {
                if let Some(dupe) = duplicates.remove(&portal) {
                    commands.entity(dupe).despawn();
                }

                continue
            };

            let map_transform = (other_portal_trns * portal_trns.inverse()).cleanup_z();
            let new_transform = map_transform * entity_trns;
            let new_local_transform = Transform::from(new_transform);

            match duplicates.entry(portal) {
                Entry::Occupied(e) => {
                    let Ok((mut local_trns, mut trns)) = transforms.get_mut(*e.get()) else {
                        commands.entity(*e.get()).despawn();
                        e.remove();

                        continue
                    };

                    *local_trns = new_local_transform;
                    *trns = new_transform;
                }
                Entry::Vacant(e) => {
                    e.insert(
                        commands
                            .spawn((mesh.clone(), material.clone(), visibility, new_transform, new_local_transform))
                            .id(),
                    );
                }
            }
        }
    }
}
