use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
};

use crate::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, init_portal_vision_mesh);
}

#[derive(Reflect, Resource, Debug, Clone)]
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
        12, 13, 14, 14, 15, 12, // left
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
