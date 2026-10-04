//! Chat's world half, handed a room of three: the speaker, a neighbour a few
//! blocks away and someone far. The room keeps what each one heard.

use chat_world::{Chat, LINE_BURST, LINE_CHARS, LINE_WINDOW, NEAR_BLOCKS, SAID, SAY, wire};
use prost::Message;
use protocol::Stance;
use topology::QuadSphere;
use world::{Level, Measure, Moment, Op, Plugin, Room, Who};

const SPEAKER: u32 = 1;
const NEIGHBOUR: u32 = 2;
const FAR: u32 = 3;

struct Three {
    now: Moment,
    here: Vec<Who>,
    heard: Vec<(u32, wire::Said)>,
}

impl Room for Three {
    fn now(&self) -> Moment {
        self.now
    }

    fn measure(&self) -> Measure {
        Measure {
            sphere: QuadSphere::new(16).unwrap(),
            moon_radius_m: 8000.0,
        }
    }

    fn sessions(&self) -> &[Who] {
        &self.here
    }

    fn tell(&mut self, kind: &str, payload: Vec<u8>, to: &[u32]) {
        assert_eq!(kind, SAID, "chat says one event");
        let said = wire::Said::decode(payload.as_slice()).unwrap();
        self.heard
            .extend(to.iter().map(|session| (*session, said.clone())));
    }
}

impl Three {
    fn texts(&self, session: u32) -> Vec<&str> {
        self.heard
            .iter()
            .filter(|(to, _)| *to == session)
            .map(|(_, said)| said.text.as_str())
            .collect()
    }

    fn heard(&self, session: u32) -> Vec<&wire::Said> {
        self.heard
            .iter()
            .filter(|(to, _)| *to == session)
            .map(|(_, said)| said)
            .collect()
    }
}

fn who(session: u32, u: f32) -> Who {
    Who {
        session,
        user: String::new(),
        name: String::new(),
        level: Level::Anonymous,
        stance: Some(Stance {
            sector: 2,
            u,
            v: 32768.0,
            ..Stance::default()
        }),
    }
}

/// The neighbour stands four blocks inside the reach, the far one four
/// blocks outside it: the warp stretches a block a little away from a sector
/// centre, so the edge is asked about with a margin.
fn three() -> (Three, Who) {
    let reach = NEAR_BLOCKS as f32;
    let speaker = who(SPEAKER, 32768.0);
    let room = Three {
        now: Moment(1_790_000_000_000),
        here: vec![
            speaker.clone(),
            who(NEIGHBOUR, 32768.0 + reach - 4.0),
            who(FAR, 32768.0 + reach + 4.0),
        ],
        heard: Vec::new(),
    };
    (room, speaker)
}

fn say(chat: &mut Chat, room: &mut Three, who: &Who, line: wire::Say) {
    chat.op(SAY, &line.encode_to_vec(), who, room);
}

fn world(text: &str) -> wire::Say {
    wire::Say {
        scope: wire::Scope::World.into(),
        text: text.into(),
        here: false,
    }
}

#[test]
fn anyone_may_speak() {
    let ops = Chat::default().ops();
    assert_eq!(
        ops,
        vec![Op {
            kind: "say",
            level: Level::Anonymous
        }],
        "chat offers one op, to every level"
    );
}

#[test]
fn a_line_reaches_its_scope() {
    let mut chat = Chat::default();
    let (mut room, speaker) = three();

    let near = wire::Say {
        scope: wire::Scope::Near.into(),
        text: "  hi  ".into(),
        here: true,
    };
    say(&mut chat, &mut room, &speaker, near);
    for session in [SPEAKER, NEIGHBOUR] {
        let heard = room.heard(session);
        assert_eq!(
            heard.len(),
            1,
            "a near line reaches the speaker and a neighbour"
        );
        assert_eq!(
            (heard[0].session, heard[0].text.as_str()),
            (SPEAKER, "hi"),
            "trimmed"
        );
        assert_eq!(heard[0].scope(), wire::Scope::Near);
        let stance = heard[0]
            .stance
            .expect("with the speaker's place, as the room holds it");
        assert_eq!(stance.u, 32768.0);
    }
    assert!(room.texts(FAR).is_empty(), "the far one hears no near line");

    say(&mut chat, &mut room, &speaker, world("all"));
    let heard = room.heard(FAR);
    assert_eq!(heard.len(), 1, "the far one hears the world line");
    assert_eq!(heard[0].text, "all");
    assert_eq!(heard[0].stance, None, "and no place unasked");
}

#[test]
fn someone_nowhere_or_on_another_body_is_not_near() {
    let mut chat = Chat::default();
    let (mut room, speaker) = three();
    room.here[1].stance = None;
    room.here[2].stance = Some(Stance {
        body: protocol::Body::Moon.into(),
        ..speaker.stance.unwrap()
    });

    let near = wire::Say {
        scope: wire::Scope::Near.into(),
        text: "hi".into(),
        here: false,
    };
    say(&mut chat, &mut room, &speaker, near);
    assert_eq!(
        room.texts(SPEAKER),
        ["hi"],
        "the speaker hears their own line"
    );
    assert!(
        room.texts(NEIGHBOUR).is_empty(),
        "one who has not said where they are"
    );
    assert!(
        room.texts(FAR).is_empty(),
        "one on the moon, at the same address"
    );
}

#[test]
fn what_is_too_long_empty_or_too_fast_is_dropped() {
    let mut chat = Chat::default();
    let (mut room, speaker) = three();

    say(
        &mut chat,
        &mut room,
        &speaker,
        world(&"x".repeat(LINE_CHARS + 1)),
    );
    say(&mut chat, &mut room, &speaker, world(""));
    chat.op(SAY, &[0xff, 0xff], &speaker, &mut room);
    chat.op("shout", &[], &speaker, &mut room);
    // Characters, not bytes: a full line of accents is still a line.
    let accented = "ç".repeat(LINE_CHARS);
    say(&mut chat, &mut room, &speaker, world(&accented));
    for i in 0..LINE_BURST + 2 {
        say(&mut chat, &mut room, &speaker, world(&i.to_string()));
    }
    assert_eq!(
        room.texts(FAR),
        [accented.as_str(), "0", "1", "2", "3"],
        "too long, empty, unread and past the rate are dropped"
    );

    // The window passes, and a session that left is counted from nothing.
    room.now = Moment(room.now.0 + LINE_WINDOW.as_millis() as u64);
    say(&mut chat, &mut room, &speaker, world("later"));
    chat.gone(SPEAKER);
    for i in 0..LINE_BURST {
        say(&mut chat, &mut room, &speaker, world(&format!("again {i}")));
    }
    let heard = room.texts(FAR);
    assert_eq!(
        heard.len(),
        5 + 1 + LINE_BURST,
        "the rate is a window, kept while a session is here"
    );
    assert_eq!(heard[5], "later");
}
