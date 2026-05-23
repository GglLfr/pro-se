use crate::{
    camera::{CameraPool, CameraPoolQuery, ClipPlane, ClipProjection, PooledCameraSystems, PrimaryCamera},
    environment::portal::{Portal, PortalLink},
    gfx::LAYER_PORTAL_RESERVE,
    prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(MaterialPlugin::<PortalVisionMaterial>::default())
        .register_asset_reflect::<PortalVisionMaterial>()
        .init_resource::<PortalVisionPool>()
        .add_systems(Startup, init_portal_vision_mesh)
        .add_systems(
            PostUpdate,
            (
                force_compute_portal_transform.in_set(PooledCameraSystems::Prepare),
                build_portal_visions.in_set(PooledCameraSystems::Obtain),
            ),
        );
}

#[derive(Reflect, Resource, Debug, Clone)]
#[reflect(Resource, Debug, Clone)]
pub struct PortalVisionMesh {
    pub mesh: Handle<Mesh>,
}

pub fn init_portal_vision_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(PortalVisionMesh {
        mesh: meshes.add({
            // Taken straight from `Cuboid`'s mesh builder implementation, except the Z coord starts at -1.
            let min = vec3(-0.5, -0.5, -1.);
            let max = vec3(0.5, 0.5, 0.);

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
                12, 13, 14, 14, 15, 12, // left
                16, 17, 18, 18, 19, 16, // top
                20, 21, 22, 22, 23, 20, // bottom
            ]);

            Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
                .with_inserted_indices(indices)
        }),
    });
}

#[derive(Reflect, Asset, AsBindGroup, Debug, Default, Clone)]
#[reflect(Debug, Default, Clone)]
#[bindless(index_table(range(50..51), binding(100)))]
pub struct PortalVisionMaterial {
    #[texture(50)]
    pub texture: Handle<Image>,
}

impl Material for PortalVisionMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path("shaders/environment/portal.wgsl".into())
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

pub fn force_compute_portal_transform(
    mut portals: Query<(&Transform, &mut GlobalTransform), (Without<ChildOf>, Changed<Transform>, Or<(With<Portal>, With<PortalVisionViewer>)>)>,
) {
    portals.par_iter_mut().for_each(|(&trns, mut global_trns)| {
        global_trns.set_if_neq(trns.into());
    });
}

#[derive(Reflect, Resource, Debug, Default)]
#[reflect(Resource, Debug, Default)]
pub struct PortalVisionPool {
    map: HashMap<AssetId<Image>, (Entity, AssetId<PortalVisionMaterial>, bool)>,
}

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(Transform)]
pub struct PortalVisionViewer;

pub fn build_portal_visions(
    mut commands: Commands,
    mut camera_pool: ResMut<CameraPool>,
    mut camera_pool_query: CameraPoolQuery,
    mut pool: ResMut<PortalVisionPool>,
    mut visibility: Query<&mut Visibility>,
    vision_mesh: Res<PortalVisionMesh>,
    mut materials: ResMut<Assets<PortalVisionMaterial>>,
    viewer: Option<Single<&GlobalTransform, With<PortalVisionViewer>>>,
    camera: Single<(&GlobalTransform, &Frustum), With<PrimaryCamera>>,
    portals: Query<(&GlobalTransform, &Aabb, &Portal, PortalLink)>,
    transforms: Query<&GlobalTransform>,
) -> Result {
    use bevy::camera::primitives::Sphere;

    let (camera_trns, frustum) = camera.into_inner();
    let viewer_trns = viewer.map(Single::into_inner).unwrap_or(camera_trns);

    for (_, material_id, visible) in pool.map.values_mut() {
        *visible = false;
        if camera_pool.needs_resize() {
            _ = materials.get_mut(*material_id);
        }
    }

    for (portal_trns, portal_aabb, portal, link) in &portals {
        let portal_affine = portal_trns.affine();
        let (scl, ..) = portal_affine.to_scale_rotation_translation();
        if scl.x.abs() < 0.00001 || scl.y.abs() < 0.00001 || scl.z.abs() < 0.00001 {
            continue
        }

        let model_sphere = Sphere {
            center: portal_affine.transform_point3a(portal_aabb.center),
            radius: portal_trns.radius_vec3a(portal_aabb.half_extents),
        };

        if !frustum.intersects_sphere(&model_sphere, false) || !frustum.intersects_obb(portal_aabb, &portal_affine, true, false) {
            continue
        }

        let viewer_to_portal = portal_affine.translation - viewer_trns.translation_vec3a();
        let orientation = viewer_to_portal.dot(portal_trns.forward().to_vec3a()).signum();

        let vision_trns = Transform {
            translation: portal_affine.translation.to_vec3(),
            scale: (scl.xy() + 2. * portal.vision_length).extend(portal.vision_length),
            ..default()
        }
        .looking_to(portal_trns.forward() * orientation, Dir3::Z);
        let vision_global_trns = GlobalTransform::from(vision_trns);

        let Ok(&other_portal_trns) = transforms.get(link.get()) else { continue };
        let other_camera_trns = other_portal_trns * camera_trns.reparented_to(portal_trns);
        let other_camera_local_trns = Transform::from(other_camera_trns);

        let other_portal_normal = other_portal_trns.forward() * orientation;
        let d = -other_portal_normal.dot(other_portal_trns.translation());

        camera_pool.obtain(&mut commands, &mut camera_pool_query, |commands, data| {
            data.camera.order = -1;
            *data.projection = Projection::custom(ClipProjection {
                clip: ClipPlane::World(HalfSpace::new(other_portal_normal.extend(d))),
                ..default()
            });

            commands.entity(data.entity).insert((other_camera_trns, other_camera_local_trns));
            match pool.map.entry(data.image.id()) {
                Entry::Occupied(occupied) => {
                    let (e, .., ref mut visible) = *occupied.into_mut();
                    *visible = true;

                    commands.entity(e).insert((vision_trns, vision_global_trns));
                }
                Entry::Vacant(vacant) => {
                    let material = materials.add(PortalVisionMaterial { texture: data.image.clone() });
                    let material_id = material.id();
                    vacant.insert((
                        commands
                            .spawn((
                                Mesh3d(vision_mesh.mesh.clone()),
                                MeshMaterial3d(material),
                                Aabb::from_min_max(vec3(-0.5, -0.5, -1.), vec3(0.5, 0.5, 1.)),
                                NoAutoAabb,
                                RenderLayers::layer(LAYER_PORTAL_RESERVE),
                                vision_trns,
                                vision_global_trns,
                            ))
                            .id(),
                        material_id,
                        true,
                    ));
                }
            }
        })?;
    }

    for &(e, .., visible) in pool.map.values() {
        // May not exist yet if it just spawned.
        if let Ok(mut vis) = visibility.get_mut(e) {
            vis.set_if_neq(match visible {
                false => Visibility::Hidden,
                true => Visibility::Inherited,
            });
        }
    }

    Ok(())
}
