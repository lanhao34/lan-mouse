//! Local edge calibration, independent of remote resolution and desktop origin.
//! This is a geometry API; capture backends and configuration do not use it yet.

/// A calibration anchor: local desktop coordinate and shared transition fraction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    pub local: f64,
    pub fraction: f64,
}

/// A monotonic piecewise-linear edge mapping. The first and last fractions
/// must be 0 and 1. Local coordinates may increase or decrease (flipped edge).
#[derive(Clone, Debug)]
pub struct EdgeMapping {
    anchors: Vec<Anchor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MappingError {
    TooFewAnchors,
    NonFinite,
    InvalidFractions,
    NonMonotonicCoordinates,
}

impl EdgeMapping {
    pub fn new(anchors: Vec<Anchor>) -> Result<Self, MappingError> {
        if anchors.len() < 2 {
            return Err(MappingError::TooFewAnchors);
        }
        if anchors
            .iter()
            .any(|a| !a.local.is_finite() || !a.fraction.is_finite())
        {
            return Err(MappingError::NonFinite);
        }
        if anchors[0].fraction != 0.0
            || anchors.last().unwrap().fraction != 1.0
            || anchors.windows(2).any(|w| w[0].fraction >= w[1].fraction)
        {
            return Err(MappingError::InvalidFractions);
        }
        let increasing = anchors[1].local > anchors[0].local;
        if anchors.windows(2).any(|w| {
            !((w[1].local - w[0].local).is_finite())
                || if increasing {
                    w[1].local <= w[0].local
                } else {
                    w[1].local >= w[0].local
                }
        }) {
            return Err(MappingError::NonMonotonicCoordinates);
        }
        Ok(Self { anchors })
    }

    /// Returns None outside the configured portal: other edges must not trigger.
    pub fn normalize(&self, coordinate: f64) -> Option<f64> {
        if !coordinate.is_finite() {
            return None;
        }
        self.anchors.windows(2).find_map(|w| {
            let (a, b) = (w[0], w[1]);
            if coordinate < a.local.min(b.local) || coordinate > a.local.max(b.local) {
                return None;
            }
            let t = (coordinate - a.local) / (b.local - a.local);
            Some(a.fraction + t * (b.fraction - a.fraction))
        })
    }

    /// Reject malformed wire values rather than warping to an arbitrary edge.
    pub fn denormalize(&self, fraction: f64) -> Option<f64> {
        if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
            return None;
        }
        self.anchors.windows(2).find_map(|w| {
            let (a, b) = (w[0], w[1]);
            if fraction < a.fraction || fraction > b.fraction {
                return None;
            }
            let t = (fraction - a.fraction) / (b.fraction - a.fraction);
            Some(a.local + t * (b.local - a.local))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mapping(points: &[(f64, f64)]) -> EdgeMapping {
        EdgeMapping::new(
            points
                .iter()
                .map(|&(local, fraction)| Anchor { local, fraction })
                .collect(),
        )
        .unwrap()
    }
    #[test]
    fn independent_peers_map_matching_relative_positions() {
        let mac = mapping(&[(-900.0, 0.0), (-100.0, 1.0)]);
        let windows = mapping(&[(200.0, 0.0), (1400.0, 1.0)]);
        assert_eq!(
            windows.denormalize(mac.normalize(-500.0).unwrap()),
            Some(800.0)
        );
        assert_eq!(mac.normalize(-901.0), None);
    }
    #[test]
    fn landmarks_and_flipped_edges_round_trip() {
        for points in [
            vec![(0.0, 0.0), (200.0, 0.5), (900.0, 1.0)],
            vec![(900.0, 0.0), (700.0, 0.5), (0.0, 1.0)],
        ] {
            let m = mapping(&points);
            for n in 0..=1000 {
                let t = f64::from(n) / 1000.0;
                assert!((m.normalize(m.denormalize(t).unwrap()).unwrap() - t).abs() < 1e-12);
            }
            assert_eq!(m.denormalize(0.5), Some(points[1].0));
        }
    }
    #[test]
    fn invalid_geometry_and_wire_values_are_rejected() {
        for points in [
            vec![(0.0, 0.0), (0.0, 1.0)],
            vec![(0.0, 0.0), (100.0, 0.5), (50.0, 1.0)],
            vec![(0.0, 0.0), (1.0, 0.0)],
        ] {
            assert!(
                EdgeMapping::new(
                    points
                        .into_iter()
                        .map(|(local, fraction)| Anchor { local, fraction })
                        .collect()
                )
                .is_err()
            );
        }
        let m = mapping(&[(0.0, 0.0), (100.0, 1.0)]);
        for value in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
            assert_eq!(m.denormalize(value), None);
        }
    }
}
