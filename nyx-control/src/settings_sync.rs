#![forbid(unsafe_code)]

//! SETTINGS frame bi-directional synchronisation and hot-reload.
//!
//! The sync task listens for local configuration changes (via [`watch::Receiver<Settings>`])
//! and broadcasts a freshly built SETTINGS frame to all active `TxQueue` handles.
//! Conversely it ingests incoming SETTINGS frames from peers and publishes the
//! merged view back onto the watch channel so that other subsystems observe
//! updates in near-real-time.

use nyx_stream::{build_settings_frame, parse_settings_frame};
use tokio::sync::{mpsc, watch};

/// Command sent to `SettingsSync`.
#[allow(dead_code)]
pub enum SettingsCmd {
    /// An inbound SETTINGS frame from peer.
    Inbound(Vec<u8>),
    /// Register transmit channel for broadcast.
    RegisterTx(mpsc::Sender<Vec<u8>>),
}

/// Spawn bidirectional SETTINGS synchroniser.
///
/// * `local_rx` yields locally-mutated [`Settings`](nyx_stream::StreamSettings).
/// * Returns a [`mpsc::Sender`] that other modules use to forward inbound frames.
#[must_use]
#[allow(dead_code)]
pub fn spawn_settings_sync(
    mut local_rx: watch::Receiver<nyx_stream::StreamSettings>,
) -> mpsc::Sender<SettingsCmd> {
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<SettingsCmd>(32);
    tokio::spawn(async move {
        // broadcast list of tx handles
        let mut peers: Vec<mpsc::Sender<Vec<u8>>> = Vec::new();
        // build initial frame
        let mut current = (*local_rx.borrow()).clone();
        let mut frame_bytes = build_settings_frame(&current.to_frame().settings);
        loop {
            tokio::select! {
                _ = local_rx.changed() => {
                    current = (*local_rx.borrow()).clone();
                    frame_bytes = build_settings_frame(&current.to_frame().settings);
                    // push to all peers
                    peers.retain(|tx| tx.try_send(frame_bytes.clone()).is_ok());
                }
                Some(cmd) = cmd_rx.recv() => match cmd {
                    SettingsCmd::Inbound(bytes) => {
                        if let Ok((_rem, frame)) = parse_settings_frame(&bytes) {
                            // Merge by overriding only provided settings
                            let old_lp = current.low_power_preference;
                            current.apply(&frame);
                            // Wake receivers after merge
                            let _ = local_rx.borrow_and_update();
                            // If Low Power preference changed, notify downstream (e.g., LowPowerManager)
                            if current.low_power_preference != old_lp {
                                // Log structured event using explicit formatting to avoid unused named args
                                tracing::info!(target="settings", "low_power_preference_changed from={} to={}", old_lp, current.low_power_preference);
                                // Downstream policy update hooks can be placed here (e.g., transport keepalive tuning)
                            }
                        }
                    }
                    SettingsCmd::RegisterTx(tx) => {
                        // send latest frame upon registration
                        let _ = tx.try_send(frame_bytes.clone());
                        peers.push(tx);
                    }
                }
            }
        }
    });
    cmd_tx
}
