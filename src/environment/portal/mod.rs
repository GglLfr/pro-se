use crate::prelude::*;

mod def;
mod render;
pub use def::*;
pub use render::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((def::plugin, render::plugin));
}
