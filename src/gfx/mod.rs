use crate::prelude::*;

mod pixelization;
pub use pixelization::*;

pub const LAYER_PORTAL_RESERVE: usize = 8;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(pixelization::plugin);
}
