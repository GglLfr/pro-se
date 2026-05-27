use crate::prelude::*;

pub trait AffineExt {
    fn as_affine(&self) -> &Affine3A;

    fn cleanup_z(&self) -> Affine3A {
        let &Affine3A { matrix3, translation } = self.as_affine();
        Affine3A {
            matrix3: Mat3A {
                x_axis: matrix3.x_axis.with_z(0.),
                y_axis: matrix3.y_axis.with_z(0.),
                z_axis: Vec3A::Z,
            },
            translation: translation.with_z(0.),
        }
    }
}

impl AffineExt for Affine3A {
    fn as_affine(&self) -> &Affine3A {
        self
    }
}

pub trait TransformExt {
    fn as_transform(&self) -> &Transform;

    fn from_affine(affine: Affine3A) -> Transform {
        let (scale, rotation, translation) = affine.to_scale_rotation_translation();
        Transform {
            scale,
            rotation,
            translation,
        }
    }
}

impl TransformExt for Transform {
    fn as_transform(&self) -> &Transform {
        self
    }
}

pub trait GlobalTransformExt {
    fn as_transform(&self) -> &GlobalTransform;

    fn inverse(&self) -> GlobalTransform {
        GlobalTransform::from(self.as_transform().affine().inverse())
    }

    fn cleanup_z(&self) -> GlobalTransform {
        GlobalTransform::from(self.as_transform().affine().cleanup_z())
    }
}

impl GlobalTransformExt for GlobalTransform {
    fn as_transform(&self) -> &GlobalTransform {
        self
    }
}
