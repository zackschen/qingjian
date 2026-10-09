//! 中英切换与修饰键事件；切换前先原样提交组合，避免把旧模式的输入带进新模式。

use objc2::DefinedClass;
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};

use super::QingjianInputController;
use crate::host;
use crate::imk::TextClient;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceKeyState(state_id: i32, key: u16) -> bool;
}

fn key_is_down(key: u16) -> bool {
    // SAFETY: 0 是 kCGEventSourceStateCombinedSessionState，只查询当前会话的键状态。
    unsafe { CGEventSourceKeyState(0, key) }
}

impl QingjianInputController {
    /// `None` 表示继续处理普通按键；其他事件直接返回是否消费。
    pub(super) fn handle_switch_event(
        &self,
        event: &NSEvent,
        client: TextClient<'_>,
    ) -> Option<bool> {
        match event.r#type() {
            NSEventType::FlagsChanged => {
                let flags = event.modifierFlags();
                let enabled =
                    host::with(|h| h.settings.config().shortcut.switch_mode.shift).unwrap_or(false);
                if !enabled {
                    self.ivars().borrow_mut().reset();
                    return Some(false);
                }
                self.ivars().borrow_mut().refresh_held_keys(key_is_down);
                let toggle = self.ivars().borrow_mut().flags_changed(
                    event.keyCode(),
                    flags.contains(NSEventModifierFlags::Shift),
                    flags.intersects(
                        NSEventModifierFlags::Command
                            | NSEventModifierFlags::Control
                            | NSEventModifierFlags::Option
                            | NSEventModifierFlags::Function,
                    ),
                );
                tracing::debug!(
                    key = event.keyCode(),
                    flags = flags.0,
                    toggle,
                    "中英切换修饰键事件"
                );
                if toggle {
                    self.commit_raw(client);
                    host::with(|h| {
                        h.english_mode = !h.english_mode;
                        h.engine.set_english_mode(false);
                        h.engine.break_chain();
                        h.cancel_prediction();
                        h.end_translation();
                        h.clear_notice();
                        h.window.hide();
                        h.indicator.update(h.english_mode);
                        tracing::info!(english = h.english_mode, "Shift 切换中英");
                    });
                }
                Some(toggle)
            }
            NSEventType::KeyDown => {
                self.ivars().borrow_mut().key_event(event.keyCode(), true);
                None
            }
            NSEventType::KeyUp => {
                self.ivars().borrow_mut().key_event(event.keyCode(), false);
                Some(false)
            }
            NSEventType::LeftMouseDown
            | NSEventType::RightMouseDown
            | NSEventType::OtherMouseDown => {
                self.ivars().borrow_mut().interrupt();
                self.commit_raw(client);
                Some(false)
            }
            _ => Some(false),
        }
    }
}
