use crate::prelude::*;

mod def;
mod physics;
mod render;
pub use def::*;
pub use physics::*;
pub use render::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((def::plugin, physics::plugin, render::plugin));
}
