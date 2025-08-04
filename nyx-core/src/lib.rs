use std::io::Write;

pub mod config;
pub mod error;
pub mod types;
#[cfg(target_os = "linux")]
pub mod sandbox;
pub mod i18n;
pub mod mobile;
pub mod push;
pub mod capability;
pub mod compliance;

pub use config::NyxConfig;
pub use config::PushProvider;
pub use config::MultipathConfig;
pub use error::NyxError;
pub use error::NyxResult;
pub use types::NodeId;
pub use types::PathId;
#[cfg(target_os = "linux")]
pub use sandbox::install_seccomp;
#[cfg(target_os = "openbsd")]
pub mod openbsd;
#[cfg(target_os = "openbsd")]
pub use openbsd::{install_pledge, unveil_path};

/// Install a panic hook that ensures `abort` so systemd captures core dump.
pub fn install_panic_abort() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("panic: {info}");
        // Flush stderr then abort using safe std::process::abort
        std::io::stderr().flush().ok();
        std::process::abort();
    }));
}

pub use capability::{Capability, FLAG_REQUIRED};
pub use compliance::{ComplianceLevel};
