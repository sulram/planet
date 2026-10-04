//! The wire messages between a client and a world server.
//!
//! Generated from `proto/planet/v1/world.proto` by `bun run proto`, which
//! writes `src/gen/`. Nothing here is written by hand except this file; the
//! same schema generates the Go side, so the two cannot drift.

pub use prost::Message;

/// The protocol a client speaks, said in `Hello`. Bumped with any change an
/// older client cannot read; the server refuses every other number.
pub const PROTOCOL: u32 = 4;

pub mod v1 {
    #![allow(clippy::all, clippy::pedantic)]
    include!("gen/planet/v1/planet.v1.rs");
}

pub use v1::*;

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
