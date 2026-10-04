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
const Protocol = 5

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
	catalog   Catalog
	installed []Installed

	mu    sync.Mutex
	actor *actor
	// Which plugins are on, by name: what the config says, then what the
	// world's admin set over it.
	on map[string]bool
}

// NewHub holds a world and the plugins its version carries, each on as the
// config says until `set` says otherwise: the world's own choice, by name
// (DECISIONS 91).
func NewHub(catalog Catalog, installed []Installed, set map[string]bool) *Hub {
	on := map[string]bool{}
	for _, in := range installed {
		on[in.Plugin.Name()] = in.On
		if chosen, said := set[in.Plugin.Name()]; said {
			on[in.Plugin.Name()] = chosen
		}
		// A system of the core is on in every world, whatever a folder says.
		if in.Core {
			on[in.Plugin.Name()] = true
		}
	}
	return &Hub{catalog: catalog, installed: installed, on: on}
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
		h.actor = newActor(h, recipe, plug(h.installed, h.on))
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

// Carries is whether this version has a plugin of that name, on or off. A
// system of the core is no plugin, and nobody switches it.
func (h *Hub) Carries(name string) bool {
	for _, in := range h.installed {
		if in.Plugin.Name() == name && !in.Core {
			return true
		}
	}
	return false
}

// Speaks is the plugins that are on, as the world's statement lists them.
func (h *Hub) Speaks() []Spoken {
	h.mu.Lock()
	defer h.mu.Unlock()
	on := make([]Spoken, 0, len(h.installed))
	for _, in := range h.installed {
		if h.on[in.Plugin.Name()] && !in.Core {
			on = append(on, Spoken{Name: in.Plugin.Name(), Version: in.Plugin.Version()})
		}
	}
	return on
}

// Switch turns a plugin on or off for this world, and tells whoever is in
// it. Keeping the choice is the caller's: the hub holds it for the process.
func (h *Hub) Switch(name string, on bool) {
	h.mu.Lock()
	h.on[name] = on
	now := make(map[string]bool, len(h.on))
	for name, on := range h.on {
		now[name] = on
	}
	a := h.actor
	h.mu.Unlock()
	if a != nil {
		// An actor that stopped reads no inbox and has nobody to tell: the
		// next one starts from what the hub holds.
		select {
		case a.inbox <- inbound{kind: switchKind, on: now}:
		case <-a.stopped:
		}
	}
}
