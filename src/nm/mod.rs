//! `NetworkManager` backend module.
//!
//! Provides [`Command`] and [`NmEvent`] types for communication between
//! the UI task and the NM service task, plus [`spawn()`] to start the backend.

pub mod demo;
pub mod error;
pub mod model;
pub mod proxies;
pub mod service;
pub mod settings;

use tokio::sync::mpsc;

pub use self::error::NmError;
pub use self::model::{AccessPoint, ApId, ConnId, ConnState};

/// Commands sent from the UI to the NM backend.
#[derive(Debug, Clone)]
pub enum Command {
    /// Trigger a `WiFi` scan.
    Scan,
    /// Connect to a known or visible AP.
    Connect {
        /// The access point to connect to.
        ap: ApId,
        /// Pre-shared key, if required.
        psk: Option<secrecy::SecretString>,
    },
    /// Connect to a hidden network by SSID.
    ConnectHidden {
        /// The SSID bytes.
        ssid: Vec<u8>,
        /// Pre-shared key, if required.
        psk: Option<secrecy::SecretString>,
    },
    /// Disconnect the active connection.
    Disconnect,
    /// Forget (delete) a saved connection.
    Forget(ConnId),
}

/// Events emitted from the NM backend to the UI.
#[derive(Debug, Clone)]
pub enum NmEvent {
    /// Full snapshot of visible access points.
    AccessPoints(Vec<AccessPoint>),
    /// Connection state changed.
    State(ConnState),
    /// A scan has started.
    ScanStarted,
    /// A scan has finished.
    ScanFinished,
    /// Result of a connection attempt.
    ConnectResult(Result<(), NmError>),
    /// Fatal backend error — cannot continue.
    Fatal(NmError),
}

/// Handle for sending commands to the backend.
#[derive(Debug, Clone)]
pub struct BackendHandle {
    /// Channel for sending commands.
    pub cmd_tx: mpsc::Sender<Command>,
}

impl BackendHandle {
    /// Send a command to the backend.
    pub async fn send(&self, cmd: Command) -> Result<(), mpsc::error::SendError<Command>> {
        self.cmd_tx.send(cmd).await
    }
}

/// Spawn the NM backend, returning a handle and event receiver.
///
/// If `demo` is true, spawns the demo backend instead of connecting to D-Bus.
pub fn spawn(demo: bool) -> (BackendHandle, mpsc::Receiver<NmEvent>) {
    let (cmd_tx, cmd_rx) = mpsc::channel(32);
    let (event_tx, event_rx) = mpsc::channel(64);

    if demo {
        tokio::spawn(demo::run(cmd_rx, event_tx));
    } else {
        tokio::spawn(service::run(cmd_rx, event_tx));
    }

    (BackendHandle { cmd_tx }, event_rx)
}
