use crate::{camera::GameCamera, prelude::*};

pub(super) fn plugin(app: &mut App) {
    app.configure_sets(PostUpdate, PooledCameraUpdateSystems.after(CameraUpdateSystems))
        .add_systems(PostUpdate, pooled_camera_system.in_set(PooledCameraUpdateSystems));
}

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(GameCamera, PooledCameraDirty)]
pub struct PooledCamera;

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[component(storage = "SparseSet")]
pub struct PooledCameraDirty;

#[derive(Reflect, SystemSet, Debug, Default, Clone, Eq, PartialEq, Hash)]
#[reflect(Debug, Default, Clone, PartialEq, Hash)]
pub struct PooledCameraUpdateSystems;

/// Calls `camera_system` for newly pooled cameras, since oftentimes pooled cameras are obtained
/// after the primary camera is updated.
pub fn pooled_camera_system(
    mut commands: Commands,
    window_resized_reader: MessageReader<WindowResized>,
    window_created_reader: MessageReader<WindowCreated>,
    window_scale_factor_changed_reader: MessageReader<WindowScaleFactorChanged>,
    image_asset_event_reader: MessageReader<AssetEvent<Image>>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<(Entity, &Window)>,
    images: Res<Assets<Image>>,
    manual_texture_views: Res<ManualTextureViews>,
    mut cameras: Query<(Entity, &mut Camera, &RenderTarget, &mut Projection), (With<PooledCamera>, With<PooledCameraDirty>)>,
) -> Result {
    // If the camera isn't despawned the next frame, it'll be updated by the regular `camera_system`
    // function, so make sure we don't update them twice.
    for (e, ..) in &cameras {
        commands.entity(e).remove::<PooledCameraDirty>();
    }

    camera_system(
        window_resized_reader,
        window_created_reader,
        window_scale_factor_changed_reader,
        image_asset_event_reader,
        primary_window,
        windows,
        images,
        manual_texture_views,
        cameras.transmute_lens::<(&mut Camera, &RenderTarget, &mut Projection)>().query(),
    )
}
