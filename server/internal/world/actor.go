package world

import (
	"time"

	"google.golang.org/protobuf/proto"

	pb "github.com/sulram/planet/server/internal/protocol"
)

// TickRate is how often stances go out, per second. A client sends its own
// at most this often too, so nothing is relayed twice in a tick.
const TickRate = 15

// heartbeat bounds the silence: an empty Stances goes out at least this
// often, so a client can tell a quiet room from a dead link.
const heartbeat = 2 * time.Second

type inboundKind uint8

const (
	joinKind inboundKind = iota
	leaveKind
	messageKind
)

type inbound struct {
	kind    inboundKind
	session *session
	message *pb.ClientMessage
}

// actor is the world actor: the one goroutine that owns an active world's
// state. Every session speaks to it through the inbox and hears from it
// through its own outbox, so nothing here needs a lock.
type actor struct {
	hub    *Hub
	id     string
	recipe Recipe
	inbox  chan inbound
	// Joins counted by the hub and not yet heard here. Guarded by hub.mu.
	pending int

	sessions map[uint32]*session
	next     uint32
	// Stances changed since the last tick, by session.
	dirty    map[uint32]*pb.Stance
	lastSent time.Time
}

func newActor(hub *Hub, id string, recipe Recipe) *actor {
	return &actor{
		hub:      hub,
		id:       id,
		recipe:   recipe,
		inbox:    make(chan inbound, 256),
		sessions: map[uint32]*session{},
		dirty:    map[uint32]*pb.Stance{},
	}
}

func (a *actor) run() {
	tick := time.NewTicker(time.Second / TickRate)
	defer tick.Stop()
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
	}}})
	a.broadcast(&pb.ServerMessage{Message: &pb.ServerMessage_Joined{Joined: &pb.Joined{Peer: s.peer()}}}, s)
}

func (a *actor) leave(s *session) {
	if _, here := a.sessions[s.id]; !here {
		return
	}
	delete(a.sessions, s.id)
	delete(a.dirty, s.id)
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
	}
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

// broadcast sends one message to every session but `except`. A session too
// slow to take it is dropped: the actor never waits for a client.
func (a *actor) broadcast(message *pb.ServerMessage, except *session) {
	frame, err := proto.Marshal(message)
	if err != nil {
		return
	}
	var slow []*session
	for _, s := range a.sessions {
		if s == except {
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
