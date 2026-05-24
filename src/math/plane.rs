use crate::prelude::*;

pub trait HalfSpaceExt {
    fn as_half_space(&self) -> HalfSpace;

    fn from_points_facing(a: Vec3A, b: Vec3A, toward: Vec3A) -> Option<HalfSpace> {
        let ab = b - a;
        let ac = toward - a;

        let normal = ab.cross(ac).cross(ab);
        if normal.length_squared() < 1e-6 {
            return None
        }

        Some(HalfSpace::new(normal.extend(-normal.dot(a))))
    }

    fn through_square(from: Vec3A, at: Affine3A) -> Option<[HalfSpace; 4]> {
        let center = at.translation;
        let x = at.matrix3.x_axis * Vec3A::splat(0.5);
        let y = at.matrix3.y_axis * Vec3A::splat(0.5);

        Some([
            Self::from_points_facing(from, center + x, center)?,
            Self::from_points_facing(from, center + y, center)?,
            Self::from_points_facing(from, center - x, center)?,
            Self::from_points_facing(from, center - y, center)?,
        ])
    }
}

impl HalfSpaceExt for HalfSpace {
    fn as_half_space(&self) -> HalfSpace {
        *self
    }
}

pub trait FrustumExt {
    fn as_frustum(&self) -> Frustum;
}

impl FrustumExt for Frustum {
    fn as_frustum(&self) -> Frustum {
        *self
    }
}
