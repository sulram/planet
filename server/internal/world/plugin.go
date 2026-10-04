package world

import (
	"slices"
	"time"

	pb "github.com/sulram/planet/server/internal/protocol"
)

// The host of plugins: who is installed, which are on in this world, and
// what an op passes on its way to its owner (DECISIONS 88, 93, 98). A
// plugin's world half is Rust in the server's module (DECISIONS 99), and
// `internal/module` hands each one over as a Plugin; the core names no
// plugin and reads no payload.

// Op is one thing a session may ask of a plugin, and the least level that
// may ask it. What a plugin offers is said as data, so a front end and an
// agent read the same list (DECISIONS 94).
type Op struct {
	Kind  string
	Level Level
}

// Who is the session an op comes from, as a plugin sees it: one of the
// core's three words, beside what and where.
type Who struct {
	Session  uint32
	Identity Identity
	// What the person is called here.
	Name string
	// Where the body stands, as the actor holds it. Nil until it has said.
	Stance *pb.Stance
}

// Room is what the host offers a plugin while it applies an op: services
// that carry no feature (DECISIONS 99).
type Room interface {
	// Now is the moment the op is applied.
	Now() time.Time
	// Recipe is the world's: what its bodies are measured by.
	Recipe() Recipe
	// Here is everyone in the world, the one who asks included, by session.
	Here() []Who
	// Tell says one event of the plugin, encoded once, to every session `to`
	// admits.
	Tell(kind string, payload []byte, to func(Who) bool)
}

// Plugin is a plugin's server half. The actor calls it, one call at a time,
// so it needs no lock of its own.
type Plugin interface {
	// Name is what the wire's envelope and the world's statement say.
	Name() string
	// Version is that of its wire and of its seam.
	Version() uint32
	// Ops is what a session may ask of it.
	Ops() []Op
	// Do applies an op the host let through: the plugin is on, the kind is
	// one of its ops and the session's level may ask it.
	Do(room Room, who Who, kind string, payload []byte)
	// Gone says a session left, so what was kept for it is dropped.
	Gone(session uint32)
}

// Installed is a plugin a version carries, and whether a world starts with
// it on.
type Installed struct {
	Plugin Plugin
	On     bool
}

// Spoken is one line of a world's statement: a plugin that is on, and its
// version.
type Spoken struct {
	Name    string `json:"name"`
	Version uint32 `json:"version"`
}

// plugged is a plugin as the actor holds it.
type plugged struct {
	plugin Plugin
	on     bool
	// The least level each op asks, by kind.
	ops map[string]Level
}

func plug(installed []Installed, on map[string]bool) []*plugged {
	held := make([]*plugged, 0, len(installed))
	for _, in := range installed {
		p := &plugged{plugin: in.Plugin, on: on[in.Plugin.Name()], ops: map[string]Level{}}
		for _, op := range in.Plugin.Ops() {
			p.ops[op.Kind] = op.Level
		}
		held = append(held, p)
	}
	return held
}

// may is the permission hook's default answer: the level an op asks. A
// plugin that answers over it, land first, is asked here (DECISIONS 93).
func may(who Who, p *plugged, kind string) bool {
	level, known := p.ops[kind]
	return known && who.Identity.Level >= level
}

// do passes one op along the path of a change: the plugin is found and on,
// the hook says the session may, and the owner applies it. An op that stops
// on the way is dropped unheard; a plugin that answers a refusal says so
// itself.
func (a *actor) do(s *session, envelope *pb.Envelope) {
	for _, p := range a.plugins {
		if p.plugin.Name() != envelope.Plugin {
			continue
		}
		if who := s.who(); p.on && may(who, p, envelope.Kind) {
			p.plugin.Do(room{actor: a, plugin: p.plugin.Name(), now: time.Now()}, who, envelope.Kind, envelope.Payload)
		}
		return
	}
}

// spoken is the plugins that are on, as the statement lists them.
func spoken(held []*plugged) []Spoken {
	on := make([]Spoken, 0, len(held))
	for _, p := range held {
		if p.on {
			on = append(on, Spoken{Name: p.plugin.Name(), Version: p.plugin.Version()})
		}
	}
	return on
}

func wirePlugins(on []Spoken) []*pb.Plugin {
	wire := make([]*pb.Plugin, 0, len(on))
	for _, p := range on {
		wire = append(wire, &pb.Plugin{Name: p.Name, Version: p.Version})
	}
	return wire
}

// room is the host as one plugin sees it, for the length of one op.
type room struct {
	actor  *actor
	plugin string
	now    time.Time
}

func (r room) Now() time.Time { return r.now }

func (r room) Tell(kind string, payload []byte, to func(Who) bool) {
	message := &pb.ServerMessage{Message: &pb.ServerMessage_Envelope{Envelope: &pb.Envelope{
		Plugin:  r.plugin,
		Kind:    kind,
		Payload: payload,
	}}}
	r.actor.relay(message, func(s *session) bool { return to(s.who()) })
}

func (r room) Recipe() Recipe { return r.actor.recipe }

func (r room) Here() []Who {
	here := make([]Who, 0, len(r.actor.sessions))
	for _, s := range r.actor.sessions {
		here = append(here, s.who())
	}
	slices.SortFunc(here, func(a, b Who) int { return int(a.Session) - int(b.Session) })
	return here
}
