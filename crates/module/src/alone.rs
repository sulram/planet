//! The module with no server around it: a test, or a host of its own. What
//! an owner asks of its store is answered from a store held here, and what
//! it keeps is written there once the call has returned, as the server does.

use std::collections::BTreeMap;
use std::sync::Mutex;

use prost::Message;
use protocol::module::{Ask, Reply, Row, Rows, ask, reply};

type Kept = BTreeMap<(String, Vec<u8>), Vec<u8>>;

static STORE: Mutex<Kept> = Mutex::new(BTreeMap::new());

/// Answers a question of an owner to its store.
pub fn asked(asked: &[u8]) -> Vec<u8> {
    let Ok(Ask {
        owner,
        ask: Some(ask),
    }) = Ask::decode(asked)
    else {
        return Vec::new();
    };
    let store = STORE.lock().expect("one call at a time");
    let rows = store
        .iter()
        .filter(|((kept_by, key), _)| {
            *kept_by == owner
                && match &ask {
                    ask::Ask::Get(wanted) => key == wanted,
                    ask::Ask::Scan(prefix) => key.starts_with(prefix),
                }
        })
        .map(|((_, key), value)| Row {
            key: key.clone(),
            value: value.clone(),
        })
        .collect();
    Rows { rows }.encode_to_vec()
}

/// Writes what a call kept.
pub fn keep(said: &[Vec<u8>]) {
    let mut store = STORE.lock().expect("one call at a time");
    for said in said {
        let Ok(Reply {
            reply: Some(reply::Reply::Keep(keep)),
        }) = Reply::decode(said.as_slice())
        else {
            continue;
        };
        match keep.forget {
            true => store.remove(&(keep.owner, keep.key)),
            false => store.insert((keep.owner, keep.key), keep.value),
        };
    }
}
