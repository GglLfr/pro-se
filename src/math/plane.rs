use crate::prelude::*;

pub trait HalfSpaceExt {
    fn as_half_space(&self) -> HalfSpace;

    fn from_point_normal(point: Vec3A, normal: Vec3A) -> HalfSpace {
        let d = -normal.dot(point);
        HalfSpace::new(normal.extend(d))
    }

    fn from_points_facing(a: Vec3A, b: Vec3A, c: Vec3A, toward: Vec3A) -> Option<HalfSpace> {
        let ab = b - a;
        let ac = c - a;

        let mut normal = ab.cross(ac).try_normalize()?;
        let mut d = -normal.dot(a);

        if normal.dot(toward) + d < 0. {
            normal = -normal;
            d = -d;
        }

        Some(HalfSpace::new(normal.extend(d)))
    }

    fn through_square(from: Vec3A, at: Affine3A) -> Option<[HalfSpace; 4]> {
        let center = at.translation;
        let x = at.matrix3.x_axis * Vec3A::splat(0.5);
        let y = at.matrix3.y_axis * Vec3A::splat(0.5);

        Some([
            Self::from_points_facing(from, center - x, center - y, center)?,
            Self::from_points_facing(from, center + x, center - y, center)?,
            Self::from_points_facing(from, center + x, center + y, center)?,
            Self::from_points_facing(from, center - x, center + y, center)?,
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

    fn cuboid(at: Affine3A, min: Vec3A, max: Vec3A) -> Frustum {
        let min = at.transform_point3a(min);
        let max = at.transform_point3a(max);

        let right = (at.matrix3.x_axis * Vec3A::ONE).normalize_or(Vec3A::X);
        let up = (at.matrix3.y_axis * Vec3A::ONE).normalize_or(Vec3A::Y);
        let back = (at.matrix3.z_axis * Vec3A::ONE).normalize_or(Vec3A::Z);

        Frustum {
            half_spaces: [
                HalfSpace::from_point_normal(min, up),
                HalfSpace::from_point_normal(min, right),
                HalfSpace::from_point_normal(max, -up),
                HalfSpace::from_point_normal(max, -right),
                HalfSpace::from_point_normal(min, -back),
                HalfSpace::from_point_normal(max, back),
            ],
        }
    }
}

impl FrustumExt for Frustum {
    fn as_frustum(&self) -> Frustum {
        *self
    }
}
