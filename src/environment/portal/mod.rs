use crate::prelude::*;

mod def;
mod physics;
mod vision;
mod vision_duplication;
pub use def::*;
pub use physics::*;
pub use vision::*;
pub use vision_duplication::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((def::plugin, physics::plugin, vision::plugin, vision_duplication::plugin));
}
