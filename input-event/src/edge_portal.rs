//! Persistable local portal selection. This draft schema is not a wire protocol.
use crate::edge_mapping::{Anchor, EdgeMapping};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortalDraft {
    pub schema_version: u32,
    pub portal_id: String,
    pub monitor_label: String,
    /// Snapshot only: must be matched and revalidated before runtime use.
    pub monitor_rect: [i32; 4],
    pub edge: Edge,
    /// Normalized local interval, inclusive, relative to the selected monitor.
    pub start: f64,
    pub end: f64,
    pub reversed: bool,
}
impl PortalDraft {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1 {
            return Err("Unsupported schema version");
        }
        if self.portal_id.trim().is_empty() || self.portal_id.len() > 128 {
            return Err("Portal ID must contain 1–128 bytes");
        }
        if self.monitor_rect[2] < 2 || self.monitor_rect[3] < 2 {
            return Err("Monitor dimensions must be at least 2 logical pixels");
        }
        if !self.start.is_finite()
            || !self.end.is_finite()
            || self.start < 0.0
            || self.end > 1.0
            || self.end - self.start + 1e-12 < 0.001
        {
            return Err("Select an ordered range of at least 0.1%");
        }
        Ok(())
    }
    pub fn mapping(&self) -> Result<EdgeMapping, &'static str> {
        self.validate()?;
        let vertical = matches!(self.edge, Edge::Left | Edge::Right);
        let origin = f64::from(self.monitor_rect[if vertical { 1 } else { 0 }]);
        let span = f64::from(self.monitor_rect[if vertical { 3 } else { 2 }] - 1);
        let mut values = [origin + self.start * span, origin + self.end * span];
        if self.reversed {
            values.swap(0, 1);
        }
        EdgeMapping::new(vec![
            Anchor {
                local: values[0],
                fraction: 0.0,
            },
            Anchor {
                local: values[1],
                fraction: 1.0,
            },
        ])
        .map_err(|_| "Invalid edge geometry")
    }
}
/// Convert drawing coordinates to a local interval fraction, clamped at edges.
pub fn drag_fraction(position: f64, origin: f64, span: f64) -> Option<f64> {
    if !position.is_finite() || !origin.is_finite() || !span.is_finite() || span <= 0.0 {
        return None;
    }
    Some(((position - origin) / span).clamp(0.0, 1.0))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn draft() -> PortalDraft {
        PortalDraft {
            schema_version: 1,
            portal_id: "mac-win".into(),
            monitor_label: "test".into(),
            monitor_rect: [-1920, -1080, 1920, 1080],
            edge: Edge::Left,
            start: 0.25,
            end: 0.75,
            reversed: false,
        }
    }
    #[test]
    fn global_coordinates_and_reverse() {
        let mut d = draft();
        let m = d.mapping().unwrap();
        assert_eq!(m.denormalize(0.0), Some(-810.25));
        assert_eq!(m.denormalize(1.0), Some(-270.75));
        d.reversed = true;
        assert_eq!(d.mapping().unwrap().denormalize(0.0), m.denormalize(1.0));
    }
    #[test]
    fn rejects_bad_imports() {
        let mut d = draft();
        d.start = f64::NAN;
        assert!(d.validate().is_err());
        d.start = 0.8;
        assert!(d.validate().is_err());
        d.start = 0.2;
        d.schema_version = 2;
        assert!(d.validate().is_err());
    }
    #[test]
    fn decimal_minimum_range_is_valid() {
        let mut d = draft();
        d.start = 0.7;
        d.end = 0.701;
        assert!(d.validate().is_ok());
    }
    #[test]
    fn drag_resize_invariance() {
        for t in [0.0, 0.2, 0.5, 1.0] {
            assert!((drag_fraction(40.0 + t * 800.0, 40.0, 800.0).unwrap() - t).abs() < 1e-12);
        }
        assert_eq!(drag_fraction(-10.0, 0.0, 20.0), Some(0.0));
        assert_eq!(drag_fraction(0.0, 0.0, 0.0), None);
    }
}
