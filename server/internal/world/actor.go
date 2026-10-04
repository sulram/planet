package world

import (
	"strings"
	"time"
	"unicode/utf8"

	"google.golang.org/protobuf/proto"

	pb "github.com/sulram/planet/server/internal/protocol"
)

// TickRate is how often stances go out, per second. A client sends its own
// at most this often too, so nothing is relayed twice in a tick.
const TickRate = 15

// heartbeat bounds the silence: an empty Stances goes out at least this
// often, so a client can tell a quiet room from a dead link.
const heartbeat = 2 * time.Second

// NameChars is the most a name carries. A UI holds the same limit.
const NameChars = 24

// cleanName is a name as the world keeps it: one line, trimmed, cut at
// NameChars. Empty is nobody in particular, which is allowed.
func cleanName(name string) string {
	if !utf8.ValidString(name) {
		return ""
	}
	name = strings.Join(strings.Fields(name), " ")
	if runes := []rune(name); len(runes) > NameChars {
		name = string(runes[:NameChars])
	}
	return strings.TrimSpace(name)
}

type inboundKind uint8

const (
	joinKind inboundKind = iota
	leaveKind
	messageKind
	// The admin switched a plugin: `on` says which are on now.
	switchKind
)

type inbound struct {
	kind    inboundKind
	session *session
	message *pb.ClientMessage
	on      map[string]bool
}

// actor is the world actor: the one goroutine that owns an active world's
// state. Every session speaks to it through the inbox and hears from it
// through its own outbox, so nothing here needs a lock.
type actor struct {
	hub    *Hub
	recipe Recipe
	inbox  chan inbound
	// Joins counted by the hub and not yet heard here. Guarded by hub.mu.
	pending int
	// Closed when the actor stops, so nobody waits on its inbox.
	stopped chan struct{}

	sessions map[uint32]*session
	next     uint32
	// Every plugin the version carries, on or off, in the config's order.
	plugins []*plugged
	// Stances changed since the last tick, by session.
	dirty    map[uint32]*pb.Stance
	lastSent time.Time
}

func newActor(hub *Hub, recipe Recipe, plugins []*plugged) *actor {
	return &actor{
		hub:      hub,
		recipe:   recipe,
		inbox:    make(chan inbound, 256),
		stopped:  make(chan struct{}),
		sessions: map[uint32]*session{},
		plugins:  plugins,
		dirty:    map[uint32]*pb.Stance{},
	}
}

func (a *actor) run() {
	tick := time.NewTicker(time.Second / TickRate)
	defer tick.Stop()
	defer close(a.stopped)
	for {
		select {
		case in := <-a.inbox:
			switch in.kind {
			case joinKind:
				a.join(in.session)
			case leaveKind:
				a.leave(in.session)
			case messageKind:
				a.handle(in.session, in.message)
			case switchKind:
				a.turn(in.on)
			}
			if len(a.sessions) == 0 && a.hub.retire(a) {
				return
			}
		case now := <-tick.C:
			a.flush(now)
		}
	}
}

func (a *actor) join(s *session) {
	a.hub.joined(a)
	a.next++
	s.id = a.next

	peers := make([]*pb.Peer, 0, len(a.sessions))
	for _, other := range a.sessions {
		peers = append(peers, other.peer())
	}
	a.sessions[s.id] = s
	s.send(&pb.ServerMessage{Message: &pb.ServerMessage_Welcome{Welcome: &pb.Welcome{
		Session: s.id,
		Recipe:  a.recipe.Wire(),
		Peers:   peers,
		Level:   s.identity.Level.wire(),
		Plugins: wirePlugins(spoken(a.plugins)),
	}}})
	a.broadcast(&pb.ServerMessage{Message: &pb.ServerMessage_Joined{Joined: &pb.Joined{Peer: s.peer()}}}, s)
}

func (a *actor) leave(s *session) {
	if _, here := a.sessions[s.id]; !here {
		return
	}
	delete(a.sessions, s.id)
	delete(a.dirty, s.id)
	for _, p := range a.plugins {
		p.plugin.Gone(s.id)
	}
	s.end()
	a.broadcast(&pb.ServerMessage{Message: &pb.ServerMessage_Left{Left: &pb.Left{Session: s.id}}}, nil)
}

func (a *actor) handle(s *session, message *pb.ClientMessage) {
	if _, here := a.sessions[s.id]; !here {
		return
	}
	switch m := message.Message.(type) {
	case *pb.ClientMessage_Stance:
		s.stance = m.Stance
		a.dirty[s.id] = m.Stance
	case *pb.ClientMessage_Wear:
		s.avatar = m.Wear.Avatar
		a.broadcast(&pb.ServerMessage{Message: &pb.ServerMessage_Wearing{Wearing: &pb.Wearing{
			Session: s.id,
			Avatar:  s.avatar,
		}}}, s)
	case *pb.ClientMessage_Envelope:
		a.do(s, m.Envelope)
	case *pb.ClientMessage_Rename:
		// An account is called what mundos signed: only a visitor takes a name here.
		if !s.identity.Visitor() {
			return
		}
		s.name = cleanName(m.Rename.Name)
		a.broadcast(&pb.ServerMessage{Message: &pb.ServerMessage_Renamed{Renamed: &pb.Renamed{
			Session: s.id,
			Name:    s.name,
		}}}, s)
	}
}

// turn takes the admin's word on which plugins are on, and says the world's
// statement again to everyone here.
func (a *actor) turn(on map[string]bool) {
	for _, p := range a.plugins {
		p.on = on[p.plugin.Name()]
	}
	a.broadcast(&pb.ServerMessage{Message: &pb.ServerMessage_Plugins{Plugins: &pb.Plugins{
		Plugins: wirePlugins(spoken(a.plugins)),
	}}}, nil)
}

// flush relays what moved since the last tick to everyone, in one frame
// encoded once. A quiet room still hears a heartbeat.
func (a *actor) flush(now time.Time) {
	if len(a.dirty) == 0 && now.Sub(a.lastSent) < heartbeat {
		return
	}
	moved := make([]*pb.Moved, 0, len(a.dirty))
	for id, stance := range a.dirty {
		moved = append(moved, &pb.Moved{Session: id, Stance: stance})
	}
	clear(a.dirty)
	a.lastSent = now
	a.broadcast(&pb.ServerMessage{Message: &pb.ServerMessage_Stances{Stances: &pb.Stances{Moved: moved}}}, nil)
}

// broadcast sends one message to every session but `except`.
func (a *actor) broadcast(message *pb.ServerMessage, except *session) {
	a.relay(message, func(s *session) bool { return s != except })
}

// relay sends one message, encoded once, to every session `to` admits. A
// session too slow to take it is dropped: the actor never waits for a client.
func (a *actor) relay(message *pb.ServerMessage, to func(*session) bool) {
	frame, err := proto.Marshal(message)
	if err != nil {
		return
	}
	var slow []*session
	for _, s := range a.sessions {
		if !to(s) {
			continue
		}
		if !s.offer(frame) {
			slow = append(slow, s)
		}
	}
	for _, s := range slow {
		a.leave(s)
	}
}
