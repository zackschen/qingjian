//! 输入源登记看门狗：系统隔一阵会忘掉第三方输入法，输入法进程自己定时检查并补登记。
//!
//! 输入法进程一直活着，系统「忘掉」青简后它仍能收到 activate / deactivate，所以由它来修最直接。
//! 检查放在子进程里（`--ensure-registration`），因为 TIS 在进程内缓存输入源列表，本进程读到的永远是旧值。

use std::time::Duration;

/// 子进程的命令行开关。
pub const ENSURE_FLAG: &str = "--ensure-registration";

/// 子进程退出码：列表里没有本输入源，已补登记。
pub const EXIT_REPAIRED: i32 = 3;

/// 两次检查的间隔：用户发现「选不了青简」到被修好，最多等这么久。
const INTERVAL: Duration = Duration::from_secs(60);

/// 起后台线程定时检查，随进程结束。
pub fn spawn() {
    let Ok(executable) = std::env::current_exe() else {
        return;
    };
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(INTERVAL);
            match std::process::Command::new(&executable)
                .arg(ENSURE_FLAG)
                .output()
            {
                Ok(output) if output.status.code() == Some(EXIT_REPAIRED) => {
                    tracing::warn!("系统输入源列表里没有青简，已重新登记");
                }
                Ok(output) if !output.status.success() => {
                    tracing::warn!(error = %String::from_utf8_lossy(&output.stderr), "输入源登记检查失败");
                }
                Ok(_) => {}
                Err(error) => tracing::warn!(%error, "无法启动输入源登记检查"),
            }
        }
    });
}
