use bevy::{
    ecs::{
        change_detection::{MaybeLocation, Tick},
        system::lifetimeless::Write,
    },
    render::extract_component::{ExtractComponent, ExtractComponentPlugin},
};

use crate::{
    camera::{GameCamera, PrimaryCamera},
    prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(ExtractComponentPlugin::<PooledCamera>::default())
        .init_resource::<CameraPool>()
        .configure_sets(
            PostUpdate,
            // Extend `CameraUpdateSystems` so it does 1) update the primary camera first, 2) pool additional cameras, and 3) what it does normally.
            (PooledCameraSystems::PrimaryUpdate, PooledCameraSystems::Obtain)
                .in_set(CameraUpdateSystems)
                .before(camera_system)
                .before(VisibilitySystems::UpdateFrusta)
                .chain(),
        )
        .add_systems(PreUpdate, free_pooled_cameras)
        .add_systems(PostUpdate, update_primary_camera.in_set(PooledCameraSystems::PrimaryUpdate));
}

pub const CAMERA_LAYER_RESERVE: usize = 16;

#[derive(Reflect, Component, ExtractComponent, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(GameCamera)]
pub struct PooledCamera;

#[derive(Reflect, SystemSet, Debug, Clone, Eq, PartialEq, Hash)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub enum PooledCameraSystems {
    PrimaryUpdate,
    Obtain,
}

/// `camera_system` and `update_frusta` combined specifically for the primary camera only.
/// This is to ensure systems that need pooling cameras can frustum cull properly.
///
/// Yes, this means the primary camera is updated twice per frame, but oh well.
//TODO 0.19 allows system removal
pub fn update_primary_camera(
    window_resized_reader: MessageReader<WindowResized>,
    window_created_reader: MessageReader<WindowCreated>,
    window_scale_factor_changed_reader: MessageReader<WindowScaleFactorChanged>,
    image_asset_event_reader: MessageReader<AssetEvent<Image>>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<(Entity, &Window)>,
    images: Res<Assets<Image>>,
    manual_texture_views: Res<ManualTextureViews>,
    mut cameras: Query<(Entity, Ref<GlobalTransform>, &mut Frustum, &mut Camera, &RenderTarget, &mut Projection), With<PrimaryCamera>>,
) -> Result {
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
    )?;

    for (_, trns, mut frustum, _, _, projection) in &mut cameras {
        if trns.is_changed() || projection.is_changed() {
            *frustum = projection.compute_frustum(&*trns);
        }
    }

    Ok(())
}

#[derive(Reflect, Resource, Debug)]
#[reflect(Resource, Debug, Default)]
pub struct CameraPool {
    allocated: Vec<Entity>,
    free: Vec<Entity>,
}

impl Default for CameraPool {
    fn default() -> Self {
        Self {
            allocated: vec![],
            free: vec![],
        }
    }
}

pub fn free_pooled_cameras(pool: ResMut<CameraPool>, mut cameras: Query<&mut Camera>) {
    let pool = pool.into_inner();
    let mut iter = cameras.iter_many_mut(&pool.allocated);
    while let Some(mut camera) = iter.fetch_next() {
        camera.is_active = false;
    }

    pool.free.append(&mut pool.allocated);
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct CameraPoolQuery {
    pub entity: Entity,
    pub camera: Write<Camera>,
    pub projection: Write<Projection>,
}

impl CameraPool {
    pub fn obtain<T>(
        &mut self,
        commands: &mut Commands,
        query: &mut Query<CameraPoolQuery>,
        apply: impl FnOnce(&mut Commands, CameraPoolQueryItem) -> T,
    ) -> Result<T> {
        if let Some(e) = self.free.pop() {
            self.allocated.push(e);

            let mut item = query.get_mut(e)?;
            item.camera.is_active = true;
            Ok(apply(commands, item))
        } else {
            let count = self.allocated.len() + self.free.len();
            let mut camera = Camera {
                order: (-1isize).saturating_sub_unsigned(count),
                ..default()
            };
            let mut projection = Projection::default();

            let entity = commands.spawn_empty().id();
            self.allocated.push(entity);

            let result = apply(commands, CameraPoolQueryItem {
                entity,
                camera: Mut::new(
                    &mut camera,
                    &mut Tick::new(0),
                    &mut Tick::new(0),
                    Tick::new(0),
                    Tick::new(0),
                    MaybeLocation::caller().as_mut(),
                ),
                projection: Mut::new(
                    &mut projection,
                    &mut Tick::new(0),
                    &mut Tick::new(0),
                    Tick::new(0),
                    Tick::new(0),
                    MaybeLocation::caller().as_mut(),
                ),
            });

            commands.entity(entity).insert((
                PooledCamera,
                RenderTarget::Window(bevy::window::WindowRef::Primary),
                RenderLayers::from_layers(&[0, CAMERA_LAYER_RESERVE + count]),
                camera,
                projection,
            ));
            Ok(result)
        }
    }
}
