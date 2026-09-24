package world

import (
	"context"
	"errors"
	"testing"
	"time"

	"google.golang.org/protobuf/proto"

	pb "github.com/sulram/planet/server/internal/protocol"
)

// pipe is a Conn made of channels: what a client sends the world reads,
// what the world writes the client reads.
type pipe struct {
	toWorld  chan []byte
	toClient chan []byte
	closed   chan struct{}
	reason   string
}

func newPipe() *pipe {
	return &pipe{toWorld: make(chan []byte, 64), toClient: make(chan []byte, 64), closed: make(chan struct{})}
}

func (p *pipe) Read(ctx context.Context) ([]byte, error) {
	select {
	case frame := <-p.toWorld:
		return frame, nil
	case <-p.closed:
		return nil, errors.New("closed")
	case <-ctx.Done():
		return nil, ctx.Err()
	}
}

func (p *pipe) Write(ctx context.Context, frame []byte) error {
	select {
	case p.toClient <- frame:
		return nil
	case <-p.closed:
		return errors.New("closed")
	case <-ctx.Done():
		return ctx.Err()
	}
}

func (p *pipe) Close(reason string) error {
	select {
	case <-p.closed:
	default:
		p.reason = reason
		close(p.closed)
	}
	return nil
}

// client drives one pipe the way a client would.
type client struct {
	t    *testing.T
	pipe *pipe
	done chan error
}

type fixedCatalog map[string]Recipe

func (c fixedCatalog) Recipe(_ context.Context, id string) (Recipe, error) {
	recipe, ok := c[id]
	if !ok {
		return Recipe{}, ErrNoWorld
	}
	return recipe, nil
}

func connect(t *testing.T, hub *Hub, worldID string, hello *pb.Hello) *client {
	return connectAs(t, hub, worldID, Identity{}, hello)
}

func connectAs(t *testing.T, hub *Hub, worldID string, identity Identity, hello *pb.Hello) *client {
	t.Helper()
	c := &client{t: t, pipe: newPipe(), done: make(chan error, 1)}
	go func() { c.done <- hub.Join(context.Background(), worldID, identity, c.pipe) }()
	c.say(&pb.ClientMessage{Message: &pb.ClientMessage_Hello{Hello: hello}})
	return c
}

func (c *client) say(message *pb.ClientMessage) {
	frame, err := proto.Marshal(message)
	if err != nil {
		c.t.Fatal(err)
	}
	c.pipe.toWorld <- frame
}

// hear waits for the next message the world sends.
func (c *client) hear() *pb.ServerMessage {
	c.t.Helper()
	select {
	case frame := <-c.pipe.toClient:
		var message pb.ServerMessage
		if err := proto.Unmarshal(frame, &message); err != nil {
			c.t.Fatal(err)
		}
		return &message
	case <-time.After(2 * time.Second):
		c.t.Fatal("heard nothing")
		return nil
	}
}

// hearUntil skips heartbeats and anything else until `want` matches.
func (c *client) hearUntil(want func(*pb.ServerMessage) bool) *pb.ServerMessage {
	c.t.Helper()
	for range 20 {
		if m := c.hear(); want(m) {
			return m
		}
	}
	c.t.Fatal("never heard what was wanted")
	return nil
}

func (c *client) leave() {
	c.pipe.Close("")
	select {
	case <-c.done:
	case <-time.After(2 * time.Second):
		c.t.Fatal("the session did not end")
	}
}

func recipe(t *testing.T) Recipe {
	t.Helper()
	r, err := NewRecipe("00000000deadbeef", 3, []byte(`{"relief_m":2000}`))
	if err != nil {
		t.Fatal(err)
	}
	return r
}

func hello() *pb.Hello {
	return &pb.Hello{Protocol: Protocol, Avatar: "avatars/Kyle.vrm"}
}

func stance(u float32) *pb.Stance {
	return &pb.Stance{Sector: 2, U: u, V: 100, HeightM: 3, FacingZ: -1, Gait: pb.Gait_GAIT_WALK, SpeedMps: 1.5}
}

func TestTwoPeopleSeeEachOther(t *testing.T) {
	hub := NewHub(fixedCatalog{"w1": recipe(t)})

	a := connect(t, hub, "w1", hello())
	welcome := a.hear().GetWelcome()
	if welcome == nil || welcome.Session == 0 || len(welcome.Peers) != 0 {
		t.Fatalf("first in an empty world: %v", welcome)
	}
	if welcome.Recipe.Seed != "00000000deadbeef" || welcome.Recipe.ParamsJson != `{"relief_m":2000}` {
		t.Fatalf("the welcome carries the recipe: %v", welcome.Recipe)
	}
	a.say(&pb.ClientMessage{Message: &pb.ClientMessage_Stance{Stance: stance(10)}})

	b := connect(t, hub, "w1", hello())
	welcomeB := b.hear().GetWelcome()
	if len(welcomeB.Peers) != 1 || welcomeB.Peers[0].Session != welcome.Session {
		t.Fatalf("the second sees the first: %v", welcomeB)
	}
	if got := welcomeB.Peers[0].Stance; got == nil || got.U != 10 {
		t.Fatalf("with where it stands: %v", got)
	}
	if welcomeB.Peers[0].Avatar != "avatars/Kyle.vrm" || !welcomeB.Peers[0].Visitor {
		t.Fatalf("a visitor in the avatar it said: %v", welcomeB.Peers[0])
	}

	joined := a.hearUntil(func(m *pb.ServerMessage) bool { return m.GetJoined() != nil }).GetJoined()
	if joined.Peer.Session != welcomeB.Session {
		t.Fatalf("the first hears the second arrive: %v", joined)
	}

	b.say(&pb.ClientMessage{Message: &pb.ClientMessage_Stance{Stance: stance(20)}})
	// A tick may still carry the first's own stance beside the second's.
	var step *pb.Moved
	a.hearUntil(func(m *pb.ServerMessage) bool {
		for _, moved := range m.GetStances().GetMoved() {
			if moved.Session == welcomeB.Session {
				step = moved
			}
		}
		return step != nil
	})
	if step.Stance.U != 20 {
		t.Fatalf("a step is relayed: %v", step)
	}

	b.say(&pb.ClientMessage{Message: &pb.ClientMessage_Wear{Wear: &pb.Wear{Avatar: "avatars/Ada.vrm"}}})
	wearing := a.hearUntil(func(m *pb.ServerMessage) bool { return m.GetWearing() != nil }).GetWearing()
	if wearing.Session != welcomeB.Session || wearing.Avatar != "avatars/Ada.vrm" {
		t.Fatalf("a change of avatar is relayed: %v", wearing)
	}

	b.leave()
	left := a.hearUntil(func(m *pb.ServerMessage) bool { return m.GetLeft() != nil }).GetLeft()
	if left.Session != welcomeB.Session {
		t.Fatalf("the first hears the second go: %v", left)
	}

	a.leave()
	deadline := time.Now().Add(2 * time.Second)
	for hub.Active() != 0 && time.Now().Before(deadline) {
		time.Sleep(10 * time.Millisecond)
	}
	if hub.Active() != 0 {
		t.Fatal("an empty world stays active")
	}
}

func TestATicketNamesAPerson(t *testing.T) {
	tickets := NewTickets()
	ticket := tickets.Mint(Identity{UserID: "u1", Name: "Ada"})
	who, ok := tickets.Redeem(ticket)
	if !ok || who.Name != "Ada" || who.Visitor() {
		t.Fatalf("a fresh ticket names its person: %v %v", who, ok)
	}
	if _, again := tickets.Redeem(ticket); again {
		t.Fatal("a ticket works once")
	}
	if _, stranger := tickets.Redeem("not-a-ticket"); stranger {
		t.Fatal("an unknown ticket names nobody")
	}

	hub := NewHub(fixedCatalog{"w1": recipe(t)})
	a := connectAs(t, hub, "w1", who, hello())
	if a.hear().GetWelcome() == nil {
		t.Fatal("a person is welcome")
	}
	b := connect(t, hub, "w1", hello())
	peer := b.hear().GetWelcome().Peers[0]
	if peer.Name != "Ada" || peer.Visitor {
		t.Fatalf("the ticket's identity is what others see: %v", peer)
	}
	a.leave()
	b.leave()
}

func TestWhatIsRefused(t *testing.T) {
	hub := NewHub(fixedCatalog{"w1": recipe(t)})

	old := connect(t, hub, "w1", &pb.Hello{Protocol: Protocol + 1})
	if old.hear().GetRefused() == nil {
		t.Fatal("another protocol is refused")
	}
	if err := <-old.done; !errors.Is(err, ErrRefused) {
		t.Fatalf("a refusal says so: %v", err)
	}

	nowhere := connect(t, hub, "nope", hello())
	if refused := nowhere.hear().GetRefused(); refused == nil || refused.Reason != ErrNoWorld.Error() {
		t.Fatalf("a world the catalog lacks is refused: %v", refused)
	}
	<-nowhere.done

	if hub.Active() != 0 {
		t.Fatal("nothing refused leaves an actor behind")
	}
}
