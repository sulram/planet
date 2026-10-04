//! The module handed what nobody means to hand it: bytes that are no call,
//! and ops of any shape from a body standing anywhere, numbers that are no
//! number among them. It answers or says nothing, and never panics: in the
//! server a panic is a trap, and a trap costs every plugin what it held.

use chat_world::wire;
use proptest::collection::vec;
use proptest::prelude::*;
use prost::Message;
use protocol::module::{Call, Op, Reply, Start, Who, call, reply};

fn start() {
    let start = call::Call::Start(Start {
        recipe: Some(protocol::Recipe::default()),
    });
    module::serve(&Call { call: Some(start) }.encode_to_vec());
}

fn stance() -> impl Strategy<Value = protocol::Stance> {
    (
        any::<i32>(),
        any::<u32>(),
        any::<f32>(),
        any::<f32>(),
        any::<f32>(),
    )
        .prop_map(|(body, sector, u, v, height_m)| protocol::Stance {
            body,
            // Half the time a sector that exists, so the measure is reached.
            sector: if sector % 2 == 0 { sector % 6 } else { sector },
            u,
            v,
            height_m,
            ..Default::default()
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4096))]

    #[test]
    fn any_bytes_are_a_call_or_nothing(call in vec(any::<u8>(), 0..2048)) {
        module::serve(&call);
    }

    #[test]
    fn a_line_of_any_shape_is_told_to_those_here_alone(
        scope in any::<i32>(),
        text in ".{0,600}",
        here in any::<bool>(),
        now_ms in any::<u64>(),
        asks in proptest::option::of(stance()),
        other in proptest::option::of(stance()),
    ) {
        start();
        let line = wire::Say { scope, text, here };
        let room = [(1, asks), (2, other), (3, None)].map(|(session, stance)| Who {
            session,
            stance,
            ..Default::default()
        });
        let op = call::Call::Op(Op {
            plugin: "chat".into(),
            kind: "say".into(),
            payload: line.encode_to_vec(),
            session: 1,
            now_ms,
            here: room.to_vec(),
        });
        for said in module::serve(&Call { call: Some(op) }.encode_to_vec()) {
            let Some(reply::Reply::Tell(tell)) = Reply::decode(said.as_slice()).unwrap().reply else {
                panic!("an op is answered with events alone");
            };
            prop_assert_eq!(tell.plugin.as_str(), "chat");
            prop_assert!(tell.to.iter().all(|session| (1..=3).contains(session)));
            // The speaker hears their own line, wherever they stand.
            prop_assert!(tell.to.contains(&1));
            let heard = wire::Said::decode(tell.payload.as_slice()).unwrap();
            prop_assert!(heard.text.chars().count() <= chat_world::LINE_CHARS);
        }
    }

    #[test]
    fn a_payload_of_any_bytes_is_a_line_or_nothing(payload in vec(any::<u8>(), 0..2048), kind in ".{0,16}") {
        start();
        let op = call::Call::Op(Op {
            plugin: "chat".into(),
            kind,
            payload,
            session: 1,
            now_ms: 1,
            here: vec![Who { session: 1, ..Default::default() }],
        });
        module::serve(&Call { call: Some(op) }.encode_to_vec());
    }
}
