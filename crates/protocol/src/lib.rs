//! The wire messages between a client and a world server.
//!
//! Generated from `proto/planet/v1/world.proto` by `bun run proto`, which
//! writes `src/gen/`, with the cells' payloads and the bridge of the
//! server's module beside it.
//! Nothing here is written by hand except this file; the same schemas
//! generate the Go side, so the two cannot drift.

pub use prost::Message;

/// The protocol a client speaks, said in `Hello`. Bumped with any change an
/// older client cannot read; the server refuses every other number.
pub const PROTOCOL: u32 = 5;

pub mod v1 {
    #![allow(clippy::all, clippy::pedantic)]
    include!("gen/planet/v1/planet.v1.rs");
}

pub use v1::*;

/// The bridge between the server and the module it runs, generated from
/// `proto/planet/module/v1/module.proto`: what the server asks of the world
/// halves and what they say back (DECISIONS 102).
pub mod module {
    pub mod v1 {
        #![allow(clippy::all, clippy::pedantic)]
        include!("gen/planet/module/v1/planet.module.v1.rs");
    }

    pub use v1::*;
}

/// The cells of a world on the wire, generated from
/// `proto/planet/cells/v1/cells.proto`: the payloads of the envelopes whose
/// owner is [`cells::OWNER`] (DECISIONS 110).
pub mod cells {
    pub mod v1 {
        #![allow(clippy::all, clippy::pedantic)]
        include!("gen/planet/cells/v1/planet.cells.v1.rs");
    }

    pub use v1::*;

    /// The name the envelope says for the cells, a system of the core.
    pub const OWNER: &str = "cells";
    /// The ops a client asks of the cells.
    pub const OPEN: &str = "open";
    pub const CHANGE: &str = "change";
    pub const CLOSE: &str = "close";
    pub const TAKE_BACK: &str = "take_back";
    pub const PUT_BACK: &str = "put_back";
    pub const LOOK: &str = "look";
    /// The events the world says of them.
    pub const OPENED: &str = "opened";
    pub const CHANGED: &str = "changed";
    pub const RESTORED: &str = "restored";
    pub const SEEN: &str = "seen";
}

/// One message as one WebSocket frame.
pub fn encode<M: Message>(message: &M) -> Vec<u8> {
    message.encode_to_vec()
}

/// A frame as a message, or why it is not one.
pub fn decode<M: Message + Default>(frame: &[u8]) -> Result<M, prost::DecodeError> {
    M::decode(frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_survives_the_wire() {
        let sent = ClientMessage {
            message: Some(client_message::Message::Stance(Stance {
                body: Body::Moon.into(),
                sector: 4,
                u: 40_000.25,
                v: 9_001.5,
                height_m: 12.0,
                facing_x: 0.0,
                facing_y: 0.0,
                facing_z: -1.0,
                gait: Gait::Run.into(),
                speed_mps: 7.5,
                sprint: true,
            })),
        };
        let frame = encode(&sent);
        assert_eq!(decode::<ClientMessage>(&frame).unwrap(), sent);
    }

    #[test]
    fn what_is_not_a_message_is_refused() {
        assert!(decode::<ServerMessage>(&[0xff, 0xff, 0xff]).is_err());
    }
}
