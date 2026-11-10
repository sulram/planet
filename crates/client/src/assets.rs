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

use std::sync::Arc;

/// A file the client wants, by asset reference, and in what form.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AssetRequest {
    pub id: u64,
    pub path: String,
    pub form: Form,
}

/// What form an answer takes. Decoding a picture is the shell's, as
/// fetching is: the browser decodes with its own codecs off the frame's
/// thread, the desktop with a codec crate, and the engine carries none.
#[derive(Clone, PartialEq, Eq)]
pub enum Form {
    /// The file's bytes, answered with [`crate::Client::asset_loaded`].
    Bytes,
    /// The file decoded as RGBA pixels, no wider than `max_side` texels on
    /// either side, answered with [`crate::Client::picture_loaded`].
    Picture { max_side: u32 },
    /// A picture inside a file the client already holds, a 3D model's: its
    /// bytes, PNG, JPEG or WebP, decoded as [`Form::Picture`] is, with nothing
    /// fetched. The request's path names the file they came from.
    Embedded { bytes: Arc<[u8]>, max_side: u32 },
}

/// The bytes of a picture said by their count: a request is logged whole.
impl core::fmt::Debug for Form {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Form::Bytes => write!(f, "Bytes"),
            Form::Picture { max_side } => write!(f, "Picture {{ max_side: {max_side} }}"),
            Form::Embedded { bytes, max_side } => write!(
                f,
                "Embedded {{ bytes: {} bytes, max_side: {max_side} }}",
                bytes.len()
            ),
        }
    }
}

/// What a pending request is for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Purpose {
    Manifest,
    /// An avatar, by the reference it was asked for.
    Avatar(String),
    Clip(crate::figure::Gait),
    /// A model's file, by its shape: `part::Shape::Vox` or `Glb`.
    Model(part::Shape),
    /// A picture inside a 3D model's file, by the file's hash and the
    /// picture's place among the model's.
    ModelPicture([u8; 32], usize),
    /// A level of a picture, by the content id of its file.
    Picture(part::ContentId),
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
        self.ask_for(path, purpose, Form::Bytes);
    }

    /// Asks for a picture as pixels, decoded no wider than `max_side`.
    pub fn ask_picture(&mut self, path: String, id: part::ContentId, max_side: u32) {
        self.ask_for(path, Purpose::Picture(id), Form::Picture { max_side });
    }

    /// Asks for a picture the client holds the bytes of decoded as pixels,
    /// no wider than `max_side`. `path` names the file they came from.
    pub fn ask_embedded(
        &mut self,
        path: String,
        purpose: Purpose,
        bytes: Arc<[u8]>,
        max_side: u32,
    ) {
        self.ask_for(path, purpose, Form::Embedded { bytes, max_side });
    }

    fn ask_for(&mut self, path: String, purpose: Purpose, form: Form) {
        self.next_id += 1;
        self.pending.push((self.next_id, purpose));
        self.outbox.push(AssetRequest {
            id: self.next_id,
            path,
            form,
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
