use crate::prelude::*;

pub const LAYER_PORTAL_RESERVE: usize = 8;

pub(super) fn plugin(app: &mut App) {
    app.register_required_components::<DirectionalLight, VolumetricLight>()
        .register_required_components::<PointLight, VolumetricLight>()
        .register_required_components::<SpotLight, VolumetricLight>()
        .add_observer(on_light_insert_enable_shadows::<DirectionalLight>)
        .add_observer(on_light_insert_enable_shadows::<PointLight>)
        .add_observer(on_light_insert_enable_shadows::<SpotLight>);
}

pub trait LightSource {
    fn shadows_enabled(&self) -> bool;

    fn set_shadows_enabled(&mut self, enabled: bool);
}

impl LightSource for DirectionalLight {
    fn shadows_enabled(&self) -> bool {
        self.shadow_maps_enabled
    }

    fn set_shadows_enabled(&mut self, enabled: bool) {
        self.shadow_maps_enabled = enabled;
    }
}

impl LightSource for PointLight {
    fn shadows_enabled(&self) -> bool {
        self.shadow_maps_enabled
    }

    fn set_shadows_enabled(&mut self, enabled: bool) {
        self.shadow_maps_enabled = enabled;
    }
}

impl LightSource for SpotLight {
    fn shadows_enabled(&self) -> bool {
        self.shadow_maps_enabled
    }

    fn set_shadows_enabled(&mut self, enabled: bool) {
        self.shadow_maps_enabled = enabled;
    }
}

pub fn on_light_insert_enable_shadows<T: LightSource + Component<Mutability = Mutable>>(insert: On<Insert, T>, mut query: Query<&mut T>) -> Result {
    query.get_mut(insert.entity)?.set_shadows_enabled(true);
    Ok(())
}
