use crate::prelude::*;

pub mod portal;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(portal::plugin);
}
