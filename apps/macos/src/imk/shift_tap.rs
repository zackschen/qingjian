//! 识别左右 Shift 的单击；其他按键、修饰键变化或会话结束都会取消待切换状态。

use std::collections::BTreeSet;

#[derive(Default)]
pub struct ShiftTap {
    down: bool,

    candidate: Option<u16>,

    held_keys: BTreeSet<u16>,
}

impl ShiftTap {
    pub fn flags_changed(&mut self, key: u16, down: bool, other_modifiers: bool) -> bool {
        let was_down = std::mem::replace(&mut self.down, down);
        let candidate = self.candidate.take();
        // 56 / 60 是 macOS 左右 Shift 的物理键码；是否启用由 shortcut.switch_mode 决定。
        if !matches!(key, 56 | 60) || other_modifiers || !self.held_keys.is_empty() {
            return false;
        }
        if !was_down && down {
            self.candidate = Some(key);
            return false;
        }
        was_down && !down && candidate == Some(key)
    }

    pub fn interrupt(&mut self) {
        self.candidate = None;
    }

    pub fn key_event(&mut self, key: u16, down: bool) {
        self.interrupt();
        if down {
            self.held_keys.insert(key);
        } else {
            self.held_keys.remove(&key);
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::ShiftTap;

    #[test]
    fn either_shift_toggles_only_on_release() {
        for key in [56, 60] {
            let mut tap = ShiftTap::default();
            assert!(!tap.flags_changed(key, true, false));
            assert!(tap.flags_changed(key, false, false));
            assert!(!tap.flags_changed(key, false, false));
        }
    }

    #[test]
    fn typing_or_clicking_cancels_the_tap() {
        let mut tap = ShiftTap::default();
        tap.flags_changed(56, true, false);
        tap.interrupt();
        assert!(!tap.flags_changed(56, false, false));
        tap.flags_changed(56, true, false);
        assert!(tap.flags_changed(56, false, false));
    }

    #[test]
    fn modifier_chords_never_toggle() {
        let mut tap = ShiftTap::default();
        assert!(!tap.flags_changed(56, true, true));
        assert!(!tap.flags_changed(56, false, true));
        tap.flags_changed(56, true, false);
        tap.flags_changed(55, true, true);
        tap.flags_changed(55, true, false);
        assert!(!tap.flags_changed(56, false, false));
    }

    #[test]
    fn caps_lock_change_and_two_shifts_cancel_the_tap() {
        let mut tap = ShiftTap::default();
        tap.flags_changed(56, true, false);
        tap.flags_changed(57, true, false);
        assert!(!tap.flags_changed(56, false, false));
        tap.flags_changed(56, true, false);
        tap.flags_changed(60, true, false);
        tap.flags_changed(56, true, false);
        assert!(!tap.flags_changed(60, false, false));
    }

    #[test]
    fn session_reset_and_unmatched_release_do_not_toggle() {
        let mut tap = ShiftTap::default();
        assert!(!tap.flags_changed(56, false, false));
        tap.flags_changed(56, true, false);
        tap.reset();
        assert!(!tap.flags_changed(56, false, false));
    }

    #[test]
    fn shift_pressed_while_another_key_is_held_does_not_toggle() {
        let mut tap = ShiftTap::default();
        tap.key_event(0, true);
        tap.flags_changed(56, true, false);
        assert!(!tap.flags_changed(56, false, false));
        tap.key_event(0, false);
        tap.flags_changed(56, true, false);
        assert!(tap.flags_changed(56, false, false));
    }
}
