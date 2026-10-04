//! The module as the server meets it, less the WASM: one call in, replies
//! out, with the plugins this version carries.

use prost::Message;
use protocol::module::{Call, Describe, Op, Reply, Start, Who, call, reply};

fn ask(call: call::Call) -> Vec<reply::Reply> {
    module::serve(&Call { call: Some(call) }.encode_to_vec())
        .iter()
        .map(|reply| Reply::decode(reply.as_slice()).unwrap().reply.unwrap())
        .collect()
}

#[test]
fn it_carries_chat_and_hosts_a_line() {
    let said = ask(call::Call::Describe(Describe {}));
    let [reply::Reply::Statement(statement)] = said.as_slice() else {
        panic!("one statement: {said:?}");
    };
    let chat = statement
        .plugins
        .iter()
        .find(|carried| carried.name == "chat")
        .expect("this version carries chat");
    assert!(chat.on, "and a world starts with it on");
    assert_eq!(chat.ops.len(), 1);
    assert_eq!(chat.ops[0].kind, "say");

    ask(call::Call::Start(Start {
        recipe: Some(protocol::Recipe {
            seed: "00000000deadbeef".into(),
            generator_version: worldgen::GENERATOR_VERSION,
            params_json: "{}".into(),
        }),
    }));
    // A line for the world, as chat's wire says it: scope 1, the text "hi".
    let line = [0x08, 0x01, 0x12, 0x02, b'h', b'i'];
    let here = [1, 2].map(|session| Who {
        session,
        ..Default::default()
    });
    let said = ask(call::Call::Op(Op {
        plugin: "chat".into(),
        kind: "say".into(),
        payload: line.to_vec(),
        session: 1,
        now_ms: 1_790_000_000_000,
        here: here.to_vec(),
    }));
    let [reply::Reply::Tell(tell)] = said.as_slice() else {
        panic!("one event: {said:?}");
    };
    assert_eq!((tell.plugin.as_str(), tell.kind.as_str()), ("chat", "said"));
    assert_eq!(tell.to, [1, 2], "a line for the world reaches everyone");
}
