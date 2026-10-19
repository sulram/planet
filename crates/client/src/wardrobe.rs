//! Loaded avatars, by asset reference. Two bodies in the same file share one
//! mesh in the renderer and one skeleton here; a file is fetched once however
//! many wear it, and dropped when nobody does, unless it is held.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use avatar::Avatar;
use scene::{SkinnedChange, SkinnedMeshId};

use crate::assets::{Purpose, Requests};
use crate::figure::Worn;

#[derive(Default)]
pub struct Wardrobe {
    loaded: HashMap<String, Worn>,
    /// Asked for and not yet answered.
    loading: HashSet<String>,
    /// Kept though nobody wears them: the offer, from the first time
    /// someone browses it, so each next press lands at once.
    held: HashSet<String>,
    /// Held and not yet asked for, in the order they will be pressed.
    queued: VecDeque<String>,
    next_mesh: u64,
    changes: Vec<SkinnedChange>,
}

impl Wardrobe {
    /// The avatar at a reference, if it has landed.
    pub fn get(&self, path: &str) -> Option<Worn> {
        self.loaded.get(path).cloned()
    }

    /// Asks for a reference nobody has asked for yet.
    pub fn want(&mut self, path: &str, requests: &mut Requests) {
        if self.loaded.contains_key(path) || !self.loading.insert(path.to_owned()) {
            return;
        }
        requests.ask(path.to_owned(), Purpose::Avatar(path.to_owned()));
    }

    /// Keeps every reference loaded whether or not a body wears it, fetched
    /// one after another in this order by [`Wardrobe::advance`], so the link
    /// goes whole to what a body waits for, then to what is pressed next.
    pub fn hold(&mut self, paths: impl IntoIterator<Item = String>) {
        for path in paths {
            if self.held.insert(path.clone()) {
                self.queued.push_back(path);
            }
        }
    }

    /// Asks for the next held reference, once nothing is on its way.
    pub fn advance(&mut self, requests: &mut Requests) {
        while self.loading.is_empty() {
            let Some(path) = self.queued.pop_front() else {
                return;
            };
            self.want(&path, requests);
        }
    }

    /// The answer to a request. A file that is not an avatar is forgotten, so
    /// asking again asks again.
    pub fn arrived(&mut self, path: &str, avatar: Result<Avatar, String>) -> Result<Worn, String> {
        self.loading.remove(path);
        let avatar = avatar?;
        self.next_mesh += 1;
        let worn = Worn {
            mesh: SkinnedMeshId(self.next_mesh),
            avatar: Arc::new(avatar),
        };
        self.changes
            .push(SkinnedChange::Add(worn.mesh, worn.avatar.mesh.clone()));
        self.loaded.insert(path.to_owned(), worn.clone());
        Ok(worn)
    }

    /// Drops every avatar no body wears and nobody holds.
    pub fn prune(&mut self, worn: impl Iterator<Item = SkinnedMeshId>) {
        let keep: HashSet<SkinnedMeshId> = worn.collect();
        self.loaded.retain(|path, worn| {
            let kept = keep.contains(&worn.mesh) || self.held.contains(path);
            if !kept {
                self.changes.push(SkinnedChange::Remove(worn.mesh));
            }
            kept
        });
    }

    pub fn drain_changes(&mut self) -> Vec<SkinnedChange> {
        core::mem::take(&mut self.changes)
    }
}
