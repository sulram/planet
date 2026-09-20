//! The asset seam: the client asks, the platform shell fetches.
//!
//! The client does no IO. It queues [`AssetRequest`]s by asset reference; a
//! shell fetches them and answers with [`crate::Client::asset_loaded`]. Same
//! code path on every platform.
//!
//! An asset reference is a relative path under the shell's asset root
//! (`avatars/Kyle.vrm`: the instance's default set) or an absolute URL (a
//! user's own upload, later). The client never tells them apart: an avatar is
//! whatever reference it was handed, and the manifest is only the set an
//! instance offers to those who have none of their own.

/// A file the client wants, by asset reference.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AssetRequest {
    pub id: u64,
    pub path: String,
}

/// What a pending request is for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Purpose {
    Manifest,
    Avatar,
    Clip(crate::figure::Gait),
}

/// `manifest.json` at the asset root: the one place that says which avatars
/// are offered and which clip plays for each gait. Edited by hand.
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub(crate) struct Manifest {
    /// Worn when the wanted avatar cannot be loaded. There is always one.
    pub default_avatar: Option<String>,
    /// What the instance offers to anyone.
    pub avatars: Vec<String>,
    pub clips: std::collections::HashMap<crate::figure::Gait, String>,
}

pub(crate) const MANIFEST_PATH: &str = "manifest.json";

#[derive(Default)]
pub(crate) struct Requests {
    next_id: u64,
    outbox: Vec<AssetRequest>,
    pending: Vec<(u64, Purpose)>,
}

impl Requests {
    pub fn ask(&mut self, path: String, purpose: Purpose) {
        // A newer avatar request replaces an older one still in flight.
        if purpose == Purpose::Avatar {
            self.pending.retain(|(_, p)| *p != Purpose::Avatar);
        }
        self.next_id += 1;
        self.pending.push((self.next_id, purpose));
        self.outbox.push(AssetRequest {
            id: self.next_id,
            path,
        });
    }

    pub fn drain(&mut self) -> Vec<AssetRequest> {
        core::mem::take(&mut self.outbox)
    }

    /// Resolves an answer. `None` when the request was superseded.
    pub fn answer(&mut self, id: u64) -> Option<Purpose> {
        let index = self
            .pending
            .iter()
            .position(|(pending, _)| *pending == id)?;
        Some(self.pending.swap_remove(index).1)
    }
}
