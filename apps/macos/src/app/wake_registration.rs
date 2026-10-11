//! 唤醒后重建输入法的系统登记；用独立子进程避开旧的 TIS 缓存，不切换当前输入源。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSWorkspace, NSWorkspaceDidWakeNotification};
use objc2_foundation::{NSObject, NSObjectProtocol};

define_class!(
    // SAFETY: NSObject 无额外子类化要求；通知在工作区主线程交付，没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    pub struct WakeRegistration;

    impl WakeRegistration {
        #[unsafe(method(didWake:))]
        fn did_wake(&self, _notification: Option<&AnyObject>) {
            let Ok(executable) = std::env::current_exe() else {
                return;
            };
            // 等待工作区恢复，不在输入法主线程上等待系统的登记服务。
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                match std::process::Command::new(executable)
                    .arg("--refresh-registration")
                    .output()
                {
                    Ok(output) if output.status.success() => {
                        tracing::info!("唤醒后已刷新输入源登记");
                    }
                    Ok(output) => {
                        tracing::warn!(error = %String::from_utf8_lossy(&output.stderr), "唤醒后刷新输入源登记失败");
                    }
                    Err(error) => tracing::warn!(%error, "无法启动输入源登记刷新"),
                }
            });
        }
    }

    unsafe impl NSObjectProtocol for WakeRegistration {}
);

impl WakeRegistration {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        // SAFETY: this 在整个应用生命周期内保留，选择器签名匹配通知回调。
        unsafe {
            center.addObserver_selector_name_object(
                &this,
                sel!(didWake:),
                Some(NSWorkspaceDidWakeNotification),
                None,
            );
        }
        this
    }
}
