use crate::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(on_portal_to_discard);
}

#[derive(Reflect, Component, Debug, Clone, Copy, PartialEq)]
#[reflect(Component, Debug, Default, Clone, PartialEq)]
#[require(Transform)]
#[component(immutable)]
pub struct Portal {
    pub size: Vec2,
    pub vision_length: f32,
}

impl Default for Portal {
    fn default() -> Self {
        Self {
            size: Vec2::ONE,
            vision_length: 1.,
        }
    }
}

fn on_portal_insert(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let size = world.get::<Portal>(entity).unwrap().size;
    world.commands().entity(entity).insert(Collider::cuboid(size.x, size.y, 0.));
}

#[derive(Reflect, Component, Debug, Clone, Copy, PartialEq, Deref)]
#[reflect(Component, Debug, Clone, PartialEq)]
//TODO 0.19 #[component(immutable, on_discard = on_portal_to_discard)]
#[relationship(relationship_target = PortalFrom)]
pub struct PortalTo(pub Entity);
impl PortalTo {
    pub const fn get(&self) -> Entity {
        self.0
    }
}

pub fn on_portal_to_discard(discard: On<Replace>, this: Query<&PortalTo>, mut commands: Commands) -> Result {
    commands.entity(this.get(discard.entity)?.get()).try_despawn();
    Ok(())
}

/*TODO 0.19 fn on_portal_to_discard(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let link = **world.get::<PortalTo>(entity).unwrap();
    world.commands().entity(link).try_despawn();
}*/

#[derive(Reflect, Component, Debug, Clone, Copy, PartialEq, Deref)]
#[reflect(Component, Debug, Clone, PartialEq)]
#[relationship_target(relationship = PortalTo, linked_spawn)]
pub struct PortalFrom(Entity);
impl PortalFrom {
    pub const fn get(&self) -> Entity {
        self.0
    }
}

#[derive(QueryData)]
pub struct PortalLink(AnyOf<(&'static PortalTo, &'static PortalFrom)>);
impl PortalLinkItem<'_, '_> {
    pub fn get(&self) -> Entity {
        match (self.0.0.map(PortalTo::get), self.0.1.map(PortalFrom::get)) {
            (Some(e), None) | (None, Some(e)) => e,
            _ => unreachable!("Entity has either none of all of PortalTo and PortalFrom. They are supposed to be mutually exclusive!"),
        }
    }
}
