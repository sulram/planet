//! The link to a world server, as the client sees it: frames in, frames out.
//!
//! The client owns the protocol and none of the socket. A platform shell
//! opens the connection, hands over every frame it reads, sends every frame
//! queued here, and says when the link is open or gone. An agent without a
//! renderer drives the same three calls.

use protocol::{ClientMessage, client_message};

/// How often a stance goes out at most, per second. The server relays at the
/// same rate, so nothing is sent twice in one tick.
pub const STANCE_HZ: f64 = 15.0;
/// A stance goes out at least this often while nothing changes, so the
/// server can tell a still body from a dead link.
const HEARTBEAT_S: f64 = 2.0;
/// The server says something at least every two seconds; nothing for this
/// long means the link is gone whatever the socket thinks.
const SILENCE_S: f64 = 10.0;

/// What the client wants the shell to do with the socket.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Outbound {
    /// One message, as one binary frame.
    Frame(Vec<u8>),
    /// Close the socket; the client is done with it.
    Close,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Link {
    Offline,
    /// Hello sent, Welcome awaited.
    Connecting,
    Online {
        session: u32,
    },
}

pub struct Session {
    link: Link,
    outbound: Vec<Outbound>,
    since_sent_s: f64,
    since_heard_s: f64,
    last_stance: Option<protocol::Stance>,
}

impl Default for Session {
    fn default() -> Session {
        Session {
            link: Link::Offline,
            outbound: Vec::new(),
            since_sent_s: 0.0,
            since_heard_s: 0.0,
            last_stance: None,
        }
    }
}

impl Session {
    pub fn link(&self) -> Link {
        self.link
    }

    pub fn online(&self) -> bool {
        matches!(self.link, Link::Online { .. })
    }

    /// The link is open: say hello.
    pub fn opened(&mut self, avatar: &str) {
        self.link = Link::Connecting;
        self.since_sent_s = 0.0;
        self.since_heard_s = 0.0;
        self.last_stance = None;
        self.say(client_message::Message::Hello(protocol::Hello {
            protocol: protocol::PROTOCOL,
            avatar: avatar.to_owned(),
        }));
    }

    pub fn welcomed(&mut self, session: u32) {
        self.link = Link::Online { session };
    }

    /// The link is gone, from either side.
    pub fn closed(&mut self) {
        self.link = Link::Offline;
        self.outbound.clear();
    }

    /// Asks the shell to end the link.
    pub fn close(&mut self) {
        self.outbound.push(Outbound::Close);
        self.link = Link::Offline;
    }

    pub fn heard(&mut self) {
        self.since_heard_s = 0.0;
    }

    /// Time passes: the stance goes out when it changed and the rate allows,
    /// or as a heartbeat. True when the server has been silent too long.
    pub fn tick(&mut self, dt: f64, stance: protocol::Stance) -> bool {
        if self.link == Link::Offline {
            return false;
        }
        self.since_sent_s += dt;
        self.since_heard_s += dt;
        if self.since_heard_s > SILENCE_S {
            return true;
        }
        if !self.online() || self.since_sent_s < 1.0 / STANCE_HZ {
            return false;
        }
        let changed = self.last_stance.as_ref() != Some(&stance);
        if changed || self.since_sent_s >= HEARTBEAT_S {
            self.since_sent_s = 0.0;
            self.last_stance = Some(stance);
            self.say(client_message::Message::Stance(stance));
        }
        false
    }

    pub fn wear(&mut self, avatar: &str) {
        if self.online() {
            self.say(client_message::Message::Wear(protocol::Wear {
                avatar: avatar.to_owned(),
            }));
        }
    }

    /// A line for whoever is in scope. Nothing while offline: a line said to
    /// nobody is not queued for later.
    pub fn say_line(&mut self, scope: protocol::Scope, text: String, here: bool) {
        if self.online() {
            self.say(client_message::Message::Say(protocol::Say {
                scope: scope.into(),
                text,
                here,
            }));
        }
    }

    fn say(&mut self, message: client_message::Message) {
        let frame = protocol::encode(&ClientMessage {
            message: Some(message),
        });
        self.outbound.push(Outbound::Frame(frame));
    }

    pub fn drain_outbound(&mut self) -> Vec<Outbound> {
        core::mem::take(&mut self.outbound)
    }
}
