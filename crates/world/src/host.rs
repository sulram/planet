//! The host of world halves (DECISIONS 99, 102, 110): what the server's
//! module is, less the names the server calls it by. It holds the systems of
//! the core that run in the world and every world half a version carries,
//! takes one call of the server at a time, hands an owner its room and gives
//! back what the owner said.
//!
//! A call and a reply cross as bytes, in the schema of
//! `proto/planet/module/v1`. A payload passes through unread. What an owner
//! reads of its store it asks the server while the call runs; what it keeps
//! goes back as replies, written when the call has returned.

use std::collections::BTreeMap;

use prost::Message;
use protocol::module::{
    Answered, Ask, Call, Carried, Keep, Offered, Reply, Rows, Statement, Tell, ask, call, reply,
};
use worldgen::Generator;

use crate::{Measure, Moment, Plugin, Room, Who};

/// A world half as a version carries it, and whether a world starts with it
/// on. The server holds the switch: the module hosts every half it carries.
pub struct Installed {
    pub plugin: Box<dyn Plugin>,
    pub on: bool,
}

/// How a recipe says the size of its bodies. `None` for a recipe that names
/// no body this version can measure.
pub type MeasureOf = fn(&protocol::Recipe) -> Option<Measure>;

/// The ground a recipe makes, to read. `None` for a recipe whose ground the
/// server does not hold.
pub type GroundOf = fn(&protocol::Recipe) -> Option<Generator>;

/// A question of an owner to its store, answered by the server at once: an
/// `Ask` in, `Rows` out, both encoded.
pub type Asked<'a> = &'a mut dyn FnMut(&[u8]) -> Vec<u8>;

/// The systems of the core and the world halves of one version.
pub struct Host {
    /// The core's systems: owners as a plugin is, always on.
    core: Vec<Box<dyn Plugin>>,
    plugged: Vec<Installed>,
    measure_of: MeasureOf,
    ground_of: GroundOf,
    /// The world the server started: its measure, and its ground where the
    /// server holds it.
    measure: Option<Measure>,
    ground: Option<Generator>,
}

impl Host {
    pub fn new(
        core: Vec<Box<dyn Plugin>>,
        plugged: Vec<Installed>,
        measure_of: MeasureOf,
        ground_of: GroundOf,
    ) -> Host {
        Host {
            core,
            plugged,
            measure_of,
            ground_of,
            measure: None,
            ground: None,
        }
    }

    /// Serves one call and gives back what was said, each reply encoded.
    /// Bytes that are no call are answered with nothing.
    pub fn serve(&mut self, call: &[u8], asked: Asked<'_>) -> Vec<Vec<u8>> {
        let Ok(Call { call: Some(call) }) = Call::decode(call) else {
            return Vec::new();
        };
        let replies = match call {
            call::Call::Describe(_) => vec![self.statement()],
            call::Call::Start(start) => {
                self.measure = start.recipe.as_ref().and_then(self.measure_of);
                self.ground = start.recipe.as_ref().and_then(self.ground_of);
                Vec::new()
            }
            call::Call::Op(op) => self.op(&op, asked),
            call::Call::Gone(gone) => {
                if let Some(owner) = self.held(&gone.plugin) {
                    owner.gone(gone.session);
                }
                Vec::new()
            }
        };
        replies
            .into_iter()
            .map(|reply| Reply { reply: Some(reply) }.encode_to_vec())
            .collect()
    }

    /// The owner of a name: a system of the core, or a world half.
    fn held(&mut self, name: &str) -> Option<&mut Box<dyn Plugin>> {
        let halves = self.plugged.iter_mut().map(|held| &mut held.plugin);
        self.core
            .iter_mut()
            .chain(halves)
            .find(|owner| owner.name() == name)
    }

    fn statement(&self) -> reply::Reply {
        let carried = |owner: &dyn Plugin, on: bool, core: bool| Carried {
            name: owner.name().to_owned(),
            version: owner.version(),
            on,
            core,
            ops: owner
                .ops()
                .into_iter()
                .map(|op| Offered {
                    kind: op.kind.to_owned(),
                    level: op.level.into(),
                })
                .collect(),
        };
        let core = self.core.iter().map(|owner| carried(&**owner, true, true));
        let halves = self
            .plugged
            .iter()
            .map(|held| carried(&*held.plugin, held.on, false));
        reply::Reply::Statement(Statement {
            plugins: core.chain(halves).collect(),
        })
    }

    /// Hands an op to its owner in a room made of the call. An op for an
    /// owner this version lacks, from a session that is not here, or before
    /// the world started, is dropped.
    fn op(&mut self, op: &protocol::module::Op, asked: Asked<'_>) -> Vec<reply::Reply> {
        let Some(measure) = self.measure else {
            return Vec::new();
        };
        let here: Vec<Who> = op.here.iter().map(who).collect();
        let Some(asks) = here.iter().find(|who| who.session == op.session).cloned() else {
            return Vec::new();
        };
        let ground = self.ground.take();
        let Some(owner) = self.held(&op.plugin) else {
            self.ground = ground;
            return Vec::new();
        };
        let mut room = Hosted {
            owner: owner.name(),
            now: Moment(op.now_ms),
            measure,
            ground: ground.as_ref(),
            here,
            said: Vec::new(),
            asked,
            written: BTreeMap::new(),
        };
        owner.op(&op.kind, &op.payload, &asks, &mut room);
        let said = room.said;
        self.ground = ground;
        said
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

/// The room of one op: what the call carried, what the owner said, and what
/// it wrote, which it reads back before the server has kept it.
struct Hosted<'a> {
    owner: &'static str,
    now: Moment,
    measure: Measure,
    ground: Option<&'a Generator>,
    here: Vec<Who>,
    said: Vec<reply::Reply>,
    asked: Asked<'a>,
    written: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
}

impl Hosted<'_> {
    fn ask(&mut self, ask: ask::Ask) -> Vec<(Vec<u8>, Vec<u8>)> {
        let ask = Ask {
            owner: self.owner.to_owned(),
            ask: Some(ask),
        };
        let rows = (self.asked)(&ask.encode_to_vec());
        let rows = Rows::decode(rows.as_slice()).unwrap_or_default();
        rows.rows
            .into_iter()
            .map(|row| (row.key, row.value))
            .collect()
    }

    fn write(&mut self, key: &[u8], value: Option<Vec<u8>>) {
        self.said.push(reply::Reply::Keep(Keep {
            owner: self.owner.to_owned(),
            key: key.to_vec(),
            value: value.clone().unwrap_or_default(),
            forget: value.is_none(),
        }));
        self.written.insert(key.to_vec(), value);
    }
}

impl Room for Hosted<'_> {
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
        self.said.push(reply::Reply::Tell(Tell {
            plugin: self.owner.to_owned(),
            kind: kind.to_owned(),
            payload,
            to: to.to_vec(),
        }));
    }

    fn ground(&self) -> Option<&Generator> {
        self.ground
    }

    fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        if let Some(written) = self.written.get(key) {
            return written.clone();
        }
        let rows = self.ask(ask::Ask::Get(key.to_vec()));
        rows.into_iter().next().map(|(_, value)| value)
    }

    fn scan(&mut self, prefix: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut rows: BTreeMap<Vec<u8>, Vec<u8>> = self
            .ask(ask::Ask::Scan(prefix.to_vec()))
            .into_iter()
            .collect();
        for (key, written) in &self.written {
            if !key.starts_with(prefix) {
                continue;
            }
            match written {
                Some(value) => rows.insert(key.clone(), value.clone()),
                None => rows.remove(key),
            };
        }
        rows.into_iter().collect()
    }

    fn keep(&mut self, key: &[u8], value: Vec<u8>) {
        self.write(key, Some(value));
    }

    fn forget(&mut self, key: &[u8]) {
        self.write(key, None);
    }

    fn refuse(&mut self, code: &str) {
        self.said.push(reply::Reply::Answered(Answered {
            code: code.to_owned(),
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Level, MeasureOf, Op};
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
        let measure: MeasureOf = |_| {
            Some(Measure {
                sphere: QuadSphere::new(16)?,
                moon_radius_m: 8000.0,
            })
        };
        Host::new(Vec::new(), vec![echo], measure, |_| None)
    }

    /// A store of nothing, which every question finds empty.
    fn nothing(_ask: &[u8]) -> Vec<u8> {
        Vec::new()
    }

    fn ask(host: &mut Host, call: call::Call) -> Vec<reply::Reply> {
        host.serve(&Call { call: Some(call) }.encode_to_vec(), &mut nothing)
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
        assert!(host.serve(&[0xff, 0xff, 0xff], &mut nothing).is_empty());
        assert!(host.serve(&[], &mut nothing).is_empty());
        let mut other = shout(1);
        if let call::Call::Op(op) = &mut other {
            op.plugin = "nobody".into();
        }
        ask(&mut host, start());
        assert!(ask(&mut host, other).is_empty(), "a plugin nobody carries");
    }
}
