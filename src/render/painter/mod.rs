mod context;
mod pipeline;
mod vertex;
pub use context::*;
pub use pipeline::*;
pub use vertex::*;

use crate::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((pipeline::plugin, vertex::plugin));
}
