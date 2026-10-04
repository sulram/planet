//! The host of world halves (DECISIONS 99, 102): what the server's module is,
//! less the names the server calls it by. It holds every world half a
//! version carries, takes one call of the server at a time, hands a world
//! half its room and gives back what the half said.
//!
//! A call and a reply cross as bytes, in the schema of
//! `proto/planet/module/v1`. A payload passes through unread.

use prost::Message;
use protocol::module::{Call, Carried, Offered, Reply, Statement, Tell, call, reply};

use crate::{Measure, Moment, Plugin, Room, Who};

/// A plugin a version carries, and whether a world starts with it on.
pub struct Installed {
    pub plugin: Box<dyn Plugin>,
    pub on: bool,
}

/// How a recipe says the size of its bodies. The module reads it with the
/// generator's own types, which this crate stands apart from.
pub type MeasureOf = fn(&protocol::Recipe) -> Option<Measure>;

/// Every world half of a version, and the world they are hosted for.
pub struct Host {
    plugged: Vec<Installed>,
    measure_of: MeasureOf,
    /// The bodies of the world that started. `None` until one has, and for
    /// a recipe whose size no body can have.
    measure: Option<Measure>,
}

impl Host {
    pub fn new(plugged: Vec<Installed>, measure_of: MeasureOf) -> Host {
        Host {
            plugged,
            measure_of,
            measure: None,
        }
    }

    /// One call of the server, and every reply to it, each encoded whole.
    /// What is no call, or names nothing held here, is answered with nothing.
    pub fn serve(&mut self, call: &[u8]) -> Vec<Vec<u8>> {
        let Ok(Call { call: Some(call) }) = Call::decode(call) else {
            return Vec::new();
        };
        let replies = match call {
            call::Call::Describe(_) => vec![self.statement()],
            call::Call::Start(start) => {
                self.measure = start.recipe.as_ref().and_then(self.measure_of);
                Vec::new()
            }
            call::Call::Op(op) => self.op(&op),
            call::Call::Gone(gone) => {
                if let Some(held) = self.held(&gone.plugin) {
                    held.plugin.gone(gone.session);
                }
                Vec::new()
            }
        };
        replies
            .into_iter()
            .map(|reply| Reply { reply: Some(reply) }.encode_to_vec())
            .collect()
    }

    /// The world half of that name, if the version carries one.
    fn held(&mut self, name: &str) -> Option<&mut Installed> {
        self.plugged
            .iter_mut()
            .find(|held| held.plugin.name() == name)
    }

    /// What the module carries, in the order the config lists it.
    fn statement(&self) -> reply::Reply {
        let plugins = self
            .plugged
            .iter()
            .map(|held| Carried {
                name: held.plugin.name().to_owned(),
                version: held.plugin.version(),
                on: held.on,
                ops: held
                    .plugin
                    .ops()
                    .into_iter()
                    .map(|op| Offered {
                        kind: op.kind.to_owned(),
                        level: op.level.into(),
                    })
                    .collect(),
            })
            .collect();
        reply::Reply::Statement(Statement { plugins })
    }

    /// Hands an op to the world half it names, in a room made of the call.
    /// An op for a world that has not started, from a session that is not in
    /// the room, or for a plugin nobody carries, is dropped.
    fn op(&mut self, op: &protocol::module::Op) -> Vec<reply::Reply> {
        let Some(measure) = self.measure else {
            return Vec::new();
        };
        let here: Vec<Who> = op.here.iter().map(who).collect();
        let Some(asks) = here.iter().find(|who| who.session == op.session).cloned() else {
            return Vec::new();
        };
        let Some(held) = self.held(&op.plugin) else {
            return Vec::new();
        };
        let mut room = Hosted {
            plugin: held.plugin.name(),
            now: Moment(op.now_ms),
            measure,
            here,
            told: Vec::new(),
        };
        held.plugin.op(&op.kind, &op.payload, &asks, &mut room);
        room.told
    }
}

fn who(who: &protocol::module::Who) -> Who {
    Who {
        session: who.session,
        user: who.user.clone(),
        name: who.name.clone(),
        level: who.level(),
        stance: who.stance,
    }
}

/// The room of one op: what the call carried, and what the half said in it.
struct Hosted {
    plugin: &'static str,
    now: Moment,
    measure: Measure,
    here: Vec<Who>,
    told: Vec<reply::Reply>,
}

impl Room for Hosted {
    fn now(&self) -> Moment {
        self.now
    }

    fn measure(&self) -> Measure {
        self.measure
    }

    fn sessions(&self) -> &[Who] {
        &self.here
    }

    fn tell(&mut self, kind: &str, payload: Vec<u8>, to: &[u32]) {
        self.told.push(reply::Reply::Tell(Tell {
            plugin: self.plugin.to_owned(),
            kind: kind.to_owned(),
            payload,
            to: to.to_vec(),
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Level, Op};
    use protocol::module::{Describe, Gone, Start};
    use topology::QuadSphere;

    /// Says back what it was asked, to everyone but those who left.
    #[derive(Default)]
    struct Echo {
        left: Vec<u32>,
    }

    impl Plugin for Echo {
        fn name(&self) -> &'static str {
            "echo"
        }

        fn version(&self) -> u32 {
            7
        }

        fn ops(&self) -> Vec<Op> {
            vec![Op {
                kind: "shout",
                level: Level::Builder,
            }]
        }

        fn op(&mut self, kind: &str, payload: &[u8], who: &Who, room: &mut dyn Room) {
            let to: Vec<u32> = room
                .sessions()
                .iter()
                .map(|other| other.session)
                .filter(|session| !self.left.contains(session))
                .collect();
            let mut said = who.name.clone().into_bytes();
            said.extend_from_slice(payload);
            room.tell(&format!("{kind}ed"), said, &to);
        }

        fn gone(&mut self, session: u32) {
            self.left.push(session);
        }
    }

    fn host() -> Host {
        let echo = Installed {
            plugin: Box::new(Echo::default()),
            on: true,
        };
        Host::new(vec![echo], |_| {
            Some(Measure {
                sphere: QuadSphere::new(16)?,
                moon_radius_m: 8000.0,
            })
        })
    }

    fn ask(host: &mut Host, call: call::Call) -> Vec<reply::Reply> {
        host.serve(&Call { call: Some(call) }.encode_to_vec())
            .iter()
            .map(|reply| Reply::decode(reply.as_slice()).unwrap().reply.unwrap())
            .collect()
    }

    fn shout(session: u32) -> call::Call {
        let here = [(1, "ana"), (2, "bo")].map(|(session, name)| protocol::module::Who {
            session,
            name: name.into(),
            ..Default::default()
        });
        call::Call::Op(protocol::module::Op {
            plugin: "echo".into(),
            kind: "shout".into(),
            payload: b"!".to_vec(),
            session,
            now_ms: 1,
            here: here.to_vec(),
        })
    }

    fn start() -> call::Call {
        call::Call::Start(Start {
            recipe: Some(protocol::Recipe::default()),
        })
    }

    #[test]
    fn it_says_what_it_carries() {
        let said = ask(&mut host(), call::Call::Describe(Describe {}));
        let [reply::Reply::Statement(statement)] = said.as_slice() else {
            panic!("one statement: {said:?}");
        };
        let [echo] = statement.plugins.as_slice() else {
            panic!("one plugin: {statement:?}");
        };
        assert_eq!(
            (echo.name.as_str(), echo.version, echo.on),
            ("echo", 7, true)
        );
        assert_eq!(echo.ops.len(), 1);
        assert_eq!(
            (echo.ops[0].kind.as_str(), echo.ops[0].level()),
            ("shout", Level::Builder)
        );
    }

    #[test]
    fn an_op_reaches_its_half_in_a_room_made_of_the_call() {
        let mut host = host();
        assert!(
            ask(&mut host, shout(1)).is_empty(),
            "a world that has not started hears no op"
        );

        ask(&mut host, start());
        let said = ask(&mut host, shout(2));
        let [reply::Reply::Tell(tell)] = said.as_slice() else {
            panic!("one event: {said:?}");
        };
        assert_eq!(
            (tell.plugin.as_str(), tell.kind.as_str()),
            ("echo", "shouted")
        );
        assert_eq!(
            tell.payload, b"bo!",
            "who asks is the session the call names"
        );
        assert_eq!(tell.to, [1, 2]);

        assert!(
            ask(&mut host, shout(9)).is_empty(),
            "one who is not in the room asks nothing"
        );
    }

    #[test]
    fn a_session_gone_is_told_to_the_half_named() {
        let mut host = host();
        ask(&mut host, start());
        let gone = |plugin: &str| {
            call::Call::Gone(Gone {
                plugin: plugin.into(),
                session: 1,
            })
        };
        ask(&mut host, gone("nobody"));
        ask(&mut host, gone("echo"));
        let said = ask(&mut host, shout(2));
        let [reply::Reply::Tell(tell)] = said.as_slice() else {
            panic!("one event: {said:?}");
        };
        assert_eq!(tell.to, [2]);
    }

    #[test]
    fn what_is_no_call_is_answered_with_nothing() {
        let mut host = host();
        assert!(host.serve(&[0xff, 0xff, 0xff]).is_empty());
        assert!(host.serve(&[]).is_empty());
        let mut other = shout(1);
        if let call::Call::Op(op) = &mut other {
            op.plugin = "nobody".into();
        }
        ask(&mut host, start());
        assert!(ask(&mut host, other).is_empty(), "a plugin nobody carries");
    }
}
