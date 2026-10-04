// Package world is the world core. The recipe and who a session is are handed
// to it through small types of its own: HTTP, mundos and the world folder
// stand outside it.
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
const Protocol = 3

// A client says Hello within this long of connecting, and then something at
// least every few seconds; a silent link is a dead one.
const (
	helloTimeout = 10 * time.Second
	readTimeout  = 15 * time.Second
)

// ErrUnfounded is what a Catalog answers while the world awaits its recipe.
var ErrUnfounded = errors.New("the world awaits its founding")

// ErrRefused wraps the reason a connection was turned away before it became
// a session: the other side heard Refused and the socket is closed.
var ErrRefused = errors.New("refused")

// Catalog is where the recipe comes from: the world folder, behind one method.
type Catalog interface {
	Recipe(ctx context.Context) (Recipe, error)
}

// Hub holds the world's actor: started on the first join and retired after
// the last leave.
type Hub struct {
	catalog Catalog

	mu    sync.Mutex
	actor *actor
}

func NewHub(catalog Catalog) *Hub {
	return &Hub{catalog: catalog}
}

// Join runs one connection as a session of the world and returns when it
// ends, from either side. Who the connection is was settled by the caller,
// from its key; a visitor is the empty identity.
func (h *Hub) Join(ctx context.Context, identity Identity, conn Conn) error {
	defer conn.Close("")

	hello, err := readHello(ctx, conn)
	if err != nil {
		return refuse(ctx, conn, err.Error())
	}
	if hello.Protocol != Protocol {
		return refuse(ctx, conn, fmt.Sprintf("protocol %d, this server speaks %d", hello.Protocol, Protocol))
	}

	a, err := h.actorFor(ctx)
	if err != nil {
		return refuse(ctx, conn, err.Error())
	}
	s := newSession(identity, hello.Avatar, cleanName(hello.Name), conn)
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

// actorFor finds the world's actor or starts it. The join it is for is
// counted as pending under the lock, so an actor that is emptying cannot
// retire between being found and hearing the join.
func (h *Hub) actorFor(ctx context.Context) (*actor, error) {
	h.mu.Lock()
	if a := h.actor; a != nil {
		a.pending++
		h.mu.Unlock()
		return a, nil
	}
	h.mu.Unlock()

	// The catalog reads a disk: not under the lock.
	recipe, err := h.catalog.Recipe(ctx)
	if err != nil {
		return nil, err
	}

	h.mu.Lock()
	defer h.mu.Unlock()
	if h.actor == nil {
		h.actor = newActor(h, recipe)
		go h.actor.run()
	}
	h.actor.pending++
	return h.actor, nil
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
	if h.actor == a {
		h.actor = nil
	}
	return true
}

// Active is whether someone is in the world right now.
func (h *Hub) Active() bool {
	h.mu.Lock()
	defer h.mu.Unlock()
	return h.actor != nil
}
