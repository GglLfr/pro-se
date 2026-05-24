use crate::prelude::*;

pub trait GlobalTransformExt {
    fn as_transform(&self) -> &GlobalTransform;

    fn inverse(&self) -> GlobalTransform {
        GlobalTransform::from(self.as_transform().affine().inverse())
    }

    fn mul(&self, other: &GlobalTransform) -> GlobalTransform {
        GlobalTransform::from(self.as_transform().affine() * other.affine())
    }
}

impl GlobalTransformExt for GlobalTransform {
    fn as_transform(&self) -> &GlobalTransform {
        self
    }
}
