//! The host of plugins on the client: who is plugged in, which of them the
//! world has on, and what each is offered (DECISIONS 88, 93, 98).
//!
//! A plugin's client half is a crate of its own that imports this one. The
//! core names no plugin: a shell plugs in what its version carries, the
//! world's statement says which are on, and a command or a frame reaches a
//! plugin by the name it gave.

use serde::Serialize;
use serde_json::Value;
use worldgen::Generator;

use crate::place::Pose;
use crate::seam::{Event, PluginOn};
use crate::session::Session;

/// A plugin's client half.
pub trait Plugin {
    /// The name the wire's envelope and the world's statement say, and the
    /// one before the dot in its commands and events: `chat.say`.
    fn name(&self) -> &'static str;

    /// The version of its wire and of its seam. A world that speaks another
    /// leaves it off here.
    fn version(&self) -> u32;

    /// A command from a front end or an agent, its name without the
    /// plugin's: `say` for `chat.say`. `body` is the rest of its JSON.
    fn command(&mut self, kind: &str, body: Value, host: &mut Host<'_>);

    /// A message of this plugin from the world server.
    fn receive(&mut self, kind: &str, payload: &[u8], host: &mut Host<'_>);
}

/// What the core offers a plugin while it handles a command or a message.
pub struct Host<'a> {
    plugin: &'static str,
    session: &'a mut Session,
    events: &'a mut Vec<Event>,
    generator: &'a Generator,
}

impl Host<'_> {
    /// Asks the world for an op of this plugin: the payload rides the core's
    /// envelope under the plugin's name. Nothing goes out while offline.
    pub fn send(&mut self, kind: &str, payload: Vec<u8>) {
        self.session.envelope(self.plugin, kind, payload);
    }

    /// Says an event of this plugin over the seam. `body` is an object, and
    /// its `type` becomes the plugin's name and `kind`: `chat.said`.
    pub fn emit<T: Serialize>(&mut self, kind: &str, body: &T) {
        match serde_json::to_value(body) {
            Ok(Value::Object(mut event)) => {
                event.insert("type".into(), format!("{}.{kind}", self.plugin).into());
                self.events.push(Event::Plugin(Value::Object(event)));
            }
            _ => self.reject(format!("the event `{kind}` is no object")),
        }
    }

    /// Refuses what was asked. The message is for logs, not for people.
    pub fn reject(&mut self, message: String) {
        self.events.push(Event::Rejected {
            message: format!("{}: {message}", self.plugin),
        });
    }

    /// A stance as a place: what a person reads out and `GoTo` takes. `None`
    /// for a stance that names nowhere on this body.
    pub fn place(&self, stance: &protocol::Stance) -> Option<String> {
        let grid = self.generator.sphere().blocks();
        Pose::of_stance(grid, stance).map(|pose| pose.place(grid))
    }
}

struct Plugged {
    plugin: Box<dyn Plugin>,
    /// Whether the world this client is in has it on, at this version.
    on: bool,
}

/// Every plugin a shell plugged in, in the order it did.
#[derive(Default)]
pub(crate) struct Plugins {
    held: Vec<Plugged>,
}

impl Plugins {
    pub(crate) fn plug(&mut self, plugin: Box<dyn Plugin>) {
        self.held.push(Plugged { plugin, on: false });
    }

    /// Takes the world's statement: a plugin is on when the world says its
    /// name at the version held here. Returns what is on now, and a line for
    /// the log about each one the world speaks in another version.
    pub(crate) fn speak(&mut self, spoken: &[protocol::Plugin]) -> (Vec<PluginOn>, Vec<String>) {
        let mut apart = Vec::new();
        for held in &mut self.held {
            let said = spoken.iter().find(|said| said.name == held.plugin.name());
            held.on = said.is_some_and(|said| said.version == held.plugin.version());
            if let Some(said) = said.filter(|_| !held.on) {
                apart.push(format!(
                    "the world speaks {} {}, this client {}",
                    said.name,
                    said.version,
                    held.plugin.version()
                ));
            }
        }
        (self.on(), apart)
    }

    /// No world has spoken: every plugin is off.
    pub(crate) fn hush(&mut self) -> Vec<PluginOn> {
        self.speak(&[]).0
    }

    fn on(&self) -> Vec<PluginOn> {
        self.held
            .iter()
            .filter(|held| held.on)
            .map(|held| PluginOn {
                name: held.plugin.name().to_owned(),
                version: held.plugin.version(),
            })
            .collect()
    }

    /// Hands a command to the plugin it names. False when no plugin of that
    /// name is on.
    pub(crate) fn command(
        &mut self,
        name: &str,
        kind: &str,
        body: Value,
        session: &mut Session,
        events: &mut Vec<Event>,
        generator: &Generator,
    ) -> bool {
        let Some(held) = self.find(name) else {
            return false;
        };
        let mut host = Host {
            plugin: held.plugin.name(),
            session,
            events,
            generator,
        };
        held.plugin.command(kind, body, &mut host);
        true
    }

    /// Hands a message from the server to the plugin its envelope names. One
    /// for a plugin that is off, or that nobody plugged in, is let pass.
    pub(crate) fn receive(
        &mut self,
        envelope: &protocol::Envelope,
        session: &mut Session,
        events: &mut Vec<Event>,
        generator: &Generator,
    ) {
        if let Some(held) = self.find(&envelope.plugin) {
            let mut host = Host {
                plugin: held.plugin.name(),
                session,
                events,
                generator,
            };
            held.plugin
                .receive(&envelope.kind, &envelope.payload, &mut host);
        }
    }

    fn find(&mut self, name: &str) -> Option<&mut Plugged> {
        self.held
            .iter_mut()
            .find(|held| held.on && held.plugin.name() == name)
    }
}
