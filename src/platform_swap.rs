//! Receiver-only automatic modifier policy. Policy changes wait for a key-up boundary.
use crate::remap::KeyRemap;
use input_event::{Event, KeyboardEvent, scancode::Linux};
use lan_mouse_proto::PeerPlatform;
use std::collections::{HashMap, HashSet};

pub(crate) fn current_platform() -> PeerPlatform {
    if cfg!(target_os = "macos") {
        PeerPlatform::MacOs
    } else if cfg!(windows) {
        PeerPlatform::Windows
    } else if cfg!(target_os = "linux") {
        PeerPlatform::Linux
    } else {
        PeerPlatform::Unknown
    }
}
fn opposite(a: PeerPlatform, b: PeerPlatform) -> bool {
    matches!(
        (a, b),
        (PeerPlatform::Windows, PeerPlatform::MacOs) | (PeerPlatform::MacOs, PeerPlatform::Windows)
    )
}
#[derive(Default)]
pub(crate) struct PeerSwap {
    pub platform: PeerPlatform,
    pub sender_remapped: bool,
    pressed: HashSet<u32>,
    masks: u32,
    effective: bool,
}
impl PeerSwap {
    pub fn metadata(&mut self, p: PeerPlatform, remapped: bool) -> bool {
        let changed = self.platform != p || self.sender_remapped != remapped;
        self.platform = p;
        self.sender_remapped = remapped;
        changed
    }
    pub fn release(&mut self) {
        self.pressed.clear();
        self.masks = 0;
        self.effective = false;
    }
    pub fn apply(&mut self, event: Event, enabled: bool, local: PeerPlatform) -> Event {
        if self.pressed.is_empty() && self.masks == 0 {
            self.effective = enabled && !self.sender_remapped && opposite(local, self.platform);
        }
        let result = if self.effective {
            swap().apply(event)
        } else {
            event
        };
        match event {
            Event::Keyboard(KeyboardEvent::Key { key, state, .. }) => {
                if state == 0 {
                    self.pressed.remove(&key);
                } else {
                    self.pressed.insert(key);
                }
            }
            Event::Keyboard(KeyboardEvent::Modifiers {
                depressed,
                latched,
                locked,
                ..
            }) => self.masks = depressed | latched | (locked & (8 | 64)),
            _ => {}
        }
        result
    }
}
fn swap() -> &'static KeyRemap {
    static SWAP: std::sync::OnceLock<KeyRemap> = std::sync::OnceLock::new();
    use Linux::*;
    SWAP.get_or_init(|| {
        KeyRemap::new(HashMap::from([
            (KeyLeftAlt, KeyLeftMeta),
            (KeyLeftMeta, KeyLeftAlt),
            (KeyRightalt, KeyRightmeta),
            (KeyRightmeta, KeyRightalt),
        ]))
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn key(k: Linux, s: u8) -> Event {
        Event::Keyboard(KeyboardEvent::Key {
            time: 7,
            key: k as u32,
            state: s,
        })
    }
    #[test]
    fn only_known_opposite_platforms_swap() {
        for local in [
            PeerPlatform::MacOs,
            PeerPlatform::Windows,
            PeerPlatform::Linux,
            PeerPlatform::Unknown,
        ] {
            for peer in [
                PeerPlatform::MacOs,
                PeerPlatform::Windows,
                PeerPlatform::Linux,
                PeerPlatform::Unknown,
            ] {
                for enabled in [false, true] {
                    let mut m = PeerSwap::default();
                    m.metadata(peer, false);
                    let expected = if enabled && opposite(local, peer) {
                        Linux::KeyLeftMeta
                    } else {
                        Linux::KeyLeftAlt
                    };
                    assert_eq!(
                        m.apply(key(Linux::KeyLeftAlt, 1), enabled, local),
                        key(expected, 1)
                    );
                }
            }
        }
    }
    #[test]
    fn both_sides_repeats_and_release_match() {
        let mut m = PeerSwap::default();
        m.metadata(PeerPlatform::Windows, false);
        for (a, b) in [
            (Linux::KeyLeftAlt, Linux::KeyLeftMeta),
            (Linux::KeyRightalt, Linux::KeyRightmeta),
        ] {
            assert_eq!(m.apply(key(a, 1), true, PeerPlatform::MacOs), key(b, 1));
            assert_eq!(m.apply(key(a, 1), true, PeerPlatform::MacOs), key(b, 1));
        }
        for (a, b) in [
            (Linux::KeyLeftAlt, Linux::KeyLeftMeta),
            (Linux::KeyRightalt, Linux::KeyRightmeta),
        ] {
            assert_eq!(m.apply(key(a, 0), false, PeerPlatform::MacOs), key(b, 0));
        }
        assert_eq!(
            m.apply(key(Linux::KeyLeftAlt, 1), false, PeerPlatform::MacOs),
            key(Linux::KeyLeftAlt, 1)
        );
    }
    #[test]
    fn delayed_platform_does_not_change_held_key() {
        let mut m = PeerSwap::default();
        assert_eq!(
            m.apply(key(Linux::KeyLeftAlt, 1), true, PeerPlatform::MacOs),
            key(Linux::KeyLeftAlt, 1)
        );
        m.metadata(PeerPlatform::Windows, false);
        assert_eq!(
            m.apply(key(Linux::KeyLeftAlt, 0), true, PeerPlatform::MacOs),
            key(Linux::KeyLeftAlt, 0)
        );
        assert_eq!(
            m.apply(key(Linux::KeyLeftAlt, 1), true, PeerPlatform::MacOs),
            key(Linux::KeyLeftMeta, 1)
        );
    }
    #[test]
    fn masks_and_other_events() {
        let mut m = PeerSwap::default();
        m.metadata(PeerPlatform::Windows, false);
        let input = Event::Keyboard(KeyboardEvent::Modifiers {
            depressed: 8,
            latched: 64,
            locked: 2,
            group: 3,
        });
        assert_eq!(
            m.apply(input, true, PeerPlatform::MacOs),
            Event::Keyboard(KeyboardEvent::Modifiers {
                depressed: 64,
                latched: 8,
                locked: 2,
                group: 3
            })
        );
        assert_eq!(
            m.apply(key(Linux::KeyA, 1), true, PeerPlatform::MacOs),
            key(Linux::KeyA, 1)
        );
    }
    #[test]
    fn manual_sender_and_disconnect_are_safe() {
        let mut m = PeerSwap::default();
        m.metadata(PeerPlatform::Windows, true);
        assert_eq!(
            m.apply(key(Linux::KeyLeftAlt, 1), true, PeerPlatform::MacOs),
            key(Linux::KeyLeftAlt, 1)
        );
        m.release();
        m.metadata(PeerPlatform::Windows, false);
        assert_eq!(
            m.apply(key(Linux::KeyLeftAlt, 1), true, PeerPlatform::MacOs),
            key(Linux::KeyLeftMeta, 1)
        );
    }
    #[test]
    fn all_four_modifiers_swap_in_both_directions() {
        for (local, peer) in [
            (PeerPlatform::MacOs, PeerPlatform::Windows),
            (PeerPlatform::Windows, PeerPlatform::MacOs),
        ] {
            let mut m = PeerSwap::default();
            m.metadata(peer, false);
            for (a, b) in [
                (Linux::KeyLeftAlt, Linux::KeyLeftMeta),
                (Linux::KeyLeftMeta, Linux::KeyLeftAlt),
                (Linux::KeyRightalt, Linux::KeyRightmeta),
                (Linux::KeyRightmeta, Linux::KeyRightalt),
            ] {
                assert_eq!(m.apply(key(a, 1), true, local), key(b, 1));
                assert_eq!(m.apply(key(a, 0), true, local), key(b, 0));
            }
        }
    }
    #[test]
    fn peer_state_is_independent_and_pointer_is_unchanged() {
        let mut win = PeerSwap::default();
        win.metadata(PeerPlatform::Windows, false);
        let mut mac = PeerSwap::default();
        mac.metadata(PeerPlatform::MacOs, false);
        assert_eq!(
            win.apply(key(Linux::KeyLeftAlt, 1), true, PeerPlatform::MacOs),
            key(Linux::KeyLeftMeta, 1)
        );
        assert_eq!(
            mac.apply(key(Linux::KeyLeftAlt, 1), true, PeerPlatform::MacOs),
            key(Linux::KeyLeftAlt, 1)
        );
        let pointer = Event::Pointer(input_event::PointerEvent::Motion {
            time: 0,
            dx: 1.5,
            dy: -3.0,
        });
        assert_eq!(win.apply(pointer, true, PeerPlatform::MacOs), pointer);
    }
}
