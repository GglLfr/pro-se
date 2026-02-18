pub mod atlas;
pub mod painter;

use crate::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((atlas::plugin, painter::plugin));
}
