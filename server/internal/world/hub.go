// Package world is the world core: the hot plane. It knows nothing about
// PocketBase; the cold plane hands it recipes and identities through small
// interfaces of ours, never the other way round.
package world

import (
	"context"
	"errors"
	"fmt"
	"sync"
	"time"

	"google.golang.org/protobuf/proto"

	pb "github.com/sulram/planet/server/internal/protocol"
)

// Protocol is the wire version this server speaks. Hello says the client's;
// any other number is refused. Kept equal to `protocol::PROTOCOL` in Rust.
const Protocol = 2

// A client says Hello within this long of connecting, and then something at
// least every few seconds; a silent link is a dead one.
const (
	helloTimeout = 10 * time.Second
	readTimeout  = 15 * time.Second
)

// ErrNoWorld is what a Catalog answers for an id it does not hold.
var ErrNoWorld = errors.New("no such world")

// ErrRefused wraps the reason a connection was turned away before it became
// a session: the other side heard Refused and the socket is closed.
var ErrRefused = errors.New("refused")

// Catalog is where recipes come from: the cold plane, behind one method.
type Catalog interface {
	Recipe(ctx context.Context, worldID string) (Recipe, error)
}

// Hub holds the active worlds: one actor each, started on the first join and
// retired after the last leave.
type Hub struct {
	catalog Catalog

	mu     sync.Mutex
	actors map[string]*actor
}

func NewHub(catalog Catalog) *Hub {
	return &Hub{catalog: catalog, actors: map[string]*actor{}}
}

// Join runs one connection as a session of a world and returns when it
// ends, from either side. Who the connection is was settled by the caller,
// from a ticket; a visitor is the empty identity.
func (h *Hub) Join(ctx context.Context, worldID string, identity Identity, conn Conn) error {
	defer conn.Close("")

	hello, err := readHello(ctx, conn)
	if err != nil {
		return refuse(ctx, conn, err.Error())
	}
	if hello.Protocol != Protocol {
		return refuse(ctx, conn, fmt.Sprintf("protocol %d, this server speaks %d", hello.Protocol, Protocol))
	}

	a, err := h.actorFor(ctx, worldID)
	if err != nil {
		return refuse(ctx, conn, err.Error())
	}
	s := newSession(identity, hello.Avatar, conn)
	a.inbox <- inbound{kind: joinKind, session: s}
	return s.run(ctx, a)
}

func readHello(ctx context.Context, conn Conn) (*pb.Hello, error) {
	ctx, cancel := context.WithTimeout(ctx, helloTimeout)
	defer cancel()
	frame, err := conn.Read(ctx)
	if err != nil {
		return nil, fmt.Errorf("no hello: %w", err)
	}
	var message pb.ClientMessage
	if err := proto.Unmarshal(frame, &message); err != nil {
		return nil, fmt.Errorf("first frame is not a message: %w", err)
	}
	hello := message.GetHello()
	if hello == nil {
		return nil, errors.New("first message is not hello")
	}
	return hello, nil
}

func refuse(ctx context.Context, conn Conn, reason string) error {
	frame, _ := proto.Marshal(&pb.ServerMessage{
		Message: &pb.ServerMessage_Refused{Refused: &pb.Refused{Reason: reason}},
	})
	_ = conn.Write(ctx, frame)
	_ = conn.Close(reason)
	return fmt.Errorf("%w: %s", ErrRefused, reason)
}

// actorFor finds a world's actor or starts one. The join it is for is
// counted as pending under the lock, so an actor that is emptying cannot
// retire between being found and hearing the join.
func (h *Hub) actorFor(ctx context.Context, worldID string) (*actor, error) {
	h.mu.Lock()
	if a, ok := h.actors[worldID]; ok {
		a.pending++
		h.mu.Unlock()
		return a, nil
	}
	h.mu.Unlock()

	// The catalog is a database: not under the lock.
	recipe, err := h.catalog.Recipe(ctx, worldID)
	if err != nil {
		return nil, err
	}

	h.mu.Lock()
	defer h.mu.Unlock()
	a, ok := h.actors[worldID]
	if !ok {
		a = newActor(h, worldID, recipe)
		h.actors[worldID] = a
		go a.run()
	}
	a.pending++
	return a, nil
}

// joined is the actor's side of the pending count.
func (h *Hub) joined(a *actor) {
	h.mu.Lock()
	a.pending--
	h.mu.Unlock()
}

// retire lets an empty actor go, unless a join is on its way. True when the
// actor is gone from the hub and may stop.
func (h *Hub) retire(a *actor) bool {
	h.mu.Lock()
	defer h.mu.Unlock()
	if a.pending > 0 {
		return false
	}
	delete(h.actors, a.id)
	return true
}

// Active is how many worlds have someone in them right now.
func (h *Hub) Active() int {
	h.mu.Lock()
	defer h.mu.Unlock()
	return len(h.actors)
}
