package world

import (
	"context"
	"errors"
	"slices"
	"strconv"
	"strings"
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

// founded is a Catalog over a world with this recipe; unfounded has none.
type founded Recipe

func (r founded) Recipe(context.Context) (Recipe, error) {
	return Recipe(r), nil
}

type unfounded struct{}

func (unfounded) Recipe(context.Context) (Recipe, error) {
	return Recipe{}, ErrUnfounded
}

func connect(t *testing.T, hub *Hub, hello *pb.Hello) *client {
	return connectAs(t, hub, Identity{}, hello)
}

func connectAs(t *testing.T, hub *Hub, identity Identity, hello *pb.Hello) *client {
	t.Helper()
	c := &client{t: t, pipe: newPipe(), done: make(chan error, 1)}
	go func() { c.done <- hub.Join(context.Background(), identity, c.pipe) }()
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
	hub := NewHub(founded(recipe(t)))

	a := connect(t, hub, hello())
	welcome := a.hear().GetWelcome()
	if welcome == nil || welcome.Session == 0 || len(welcome.Peers) != 0 {
		t.Fatalf("first in an empty world: %v", welcome)
	}
	if welcome.Recipe.Seed != "00000000deadbeef" || welcome.Recipe.ParamsJson != `{"relief_m":2000}` {
		t.Fatalf("the welcome carries the recipe: %v", welcome.Recipe)
	}
	a.say(&pb.ClientMessage{Message: &pb.ClientMessage_Stance{Stance: stance(10)}})
	// The stance lands in the actor by its own road, so it is heard back
	// before the second person arrives to be told it.
	a.hearUntil(func(m *pb.ServerMessage) bool { return len(m.GetStances().GetMoved()) > 0 })

	b := connect(t, hub, hello())
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
	for hub.Active() && time.Now().Before(deadline) {
		time.Sleep(10 * time.Millisecond)
	}
	if hub.Active() {
		t.Fatal("an empty world stays active")
	}
}

func TestAKeyNamesAPerson(t *testing.T) {
	now := time.Unix(1790000000, 0)
	keys := newKeys(func() time.Time { return now })
	key := keys.Mint(Identity{UserID: "u1", Name: "Ada", Level: LevelBuilder})
	who, ok := keys.Find(key)
	if !ok || who.Name != "Ada" || who.Level != LevelBuilder || who.Visitor() {
		t.Fatalf("a key names its person and their level: %v %v", who, ok)
	}
	if _, again := keys.Find(key); !again {
		t.Fatal("a key stands for the next link too")
	}
	if _, stranger := keys.Find("not-a-key"); stranger {
		t.Fatal("an unknown key names nobody")
	}
	now = now.Add(KeyLife)
	if _, late := keys.Find(key); late {
		t.Fatal("a key past its life names nobody")
	}

	hub := NewHub(founded(recipe(t)))
	a := connectAs(t, hub, who, hello())
	if welcome := a.hear().GetWelcome(); welcome == nil || welcome.Level != pb.Level_LEVEL_BUILDER {
		t.Fatalf("a person is welcome, and told their level: %v", welcome)
	}
	b := connect(t, hub, hello())
	welcome := b.hear().GetWelcome()
	if peer := welcome.Peers[0]; peer.Name != "Ada" || peer.Visitor {
		t.Fatalf("the key's identity is what others see: %v", peer)
	}
	if welcome.Level != pb.Level_LEVEL_ANONYMOUS {
		t.Fatalf("a visitor is told it is anonymous: %v", welcome.Level)
	}
	a.leave()
	b.leave()
}

// The wire counts levels as the core does: a level is cast, never mapped.
func TestTheWireCountsLevelsAsTheCoreDoes(t *testing.T) {
	for level, name := range levelNames {
		wire := "LEVEL_" + strings.ToUpper(name)
		if got := Level(level).wire().String(); got != wire {
			t.Fatalf("level %d is %s on the wire, and %s in the core", level, got, wire)
		}
		if parsed, ok := ParseLevel(name); !ok || parsed != Level(level) {
			t.Fatalf("%s reads back as %v", name, parsed)
		}
	}
	if _, ok := ParseLevel("superadmin"); ok {
		t.Fatal("a word mundos does not write is no level")
	}
}

func TestWhatIsRefused(t *testing.T) {
	hub := NewHub(founded(recipe(t)))

	old := connect(t, hub, &pb.Hello{Protocol: Protocol + 1})
	if old.hear().GetRefused() == nil {
		t.Fatal("another protocol is refused")
	}
	if err := <-old.done; !errors.Is(err, ErrRefused) {
		t.Fatalf("a refusal says so: %v", err)
	}

	if hub.Active() {
		t.Fatal("nothing refused leaves an actor behind")
	}

	empty := NewHub(unfounded{})
	early := connect(t, empty, hello())
	if refused := early.hear().GetRefused(); refused == nil || refused.Reason != ErrUnfounded.Error() {
		t.Fatalf("a world with no recipe yet is refused: %v", refused)
	}
	<-early.done
	if empty.Active() {
		t.Fatal("an unfounded world starts no actor")
	}
}

func TestANameIsGivenAndChanged(t *testing.T) {
	hub := NewHub(founded(recipe(t)))
	a := connect(t, hub, &pb.Hello{Protocol: Protocol, Name: "  Zed   the  " + strings.Repeat("z", NameChars)})
	a.hear()
	b := connect(t, hub, hello())
	peer := b.hear().GetWelcome().Peers[0]
	if peer.Name != "Zed the "+strings.Repeat("z", NameChars-8) || !peer.Visitor {
		t.Fatalf("a visitor is called what it said, on one line and cut at NameChars: %q", peer.Name)
	}

	a.say(&pb.ClientMessage{Message: &pb.ClientMessage_Rename{Rename: &pb.Rename{Name: "Ada"}}})
	renamed := b.hearUntil(func(m *pb.ServerMessage) bool { return m.GetRenamed() != nil }).GetRenamed()
	if renamed.Session != peer.Session || renamed.Name != "Ada" {
		t.Fatalf("a new name is relayed: %v", renamed)
	}

	// An account's name is the one mundos signed: neither the hello nor a
	// rename changes it.
	c := connectAs(t, hub, Identity{UserID: "u1", Name: "Grace"}, &pb.Hello{Protocol: Protocol, Name: "Impostor"})
	c.hear()
	joined := b.hearUntil(func(m *pb.ServerMessage) bool { return m.GetJoined() != nil }).GetJoined()
	if joined.Peer.Name != "Grace" {
		t.Fatalf("the account names the person: %v", joined.Peer)
	}
	c.say(&pb.ClientMessage{Message: &pb.ClientMessage_Rename{Rename: &pb.Rename{Name: "Impostor"}}})
	a.say(&pb.ClientMessage{Message: &pb.ClientMessage_Rename{Rename: &pb.Rename{Name: "Bob"}}})
	// The visitor's rename is heard next: the account's was dropped before it.
	if next := b.hearUntil(func(m *pb.ServerMessage) bool { return m.GetRenamed() != nil }).GetRenamed(); next.Name != "Bob" {
		t.Fatalf("an account is not renamed in the world: %v", next)
	}
	a.leave()
	b.leave()
	c.leave()
}

func TestALineReachesItsScope(t *testing.T) {
	hub := NewHub(founded(recipe(t)))
	a := connect(t, hub, hello())
	me := a.hear().GetWelcome().Session
	b := connect(t, hub, hello())
	b.hear()
	far := connect(t, hub, hello())
	far.hear()
	a.say(&pb.ClientMessage{Message: &pb.ClientMessage_Stance{Stance: stance(100)}})
	b.say(&pb.ClientMessage{Message: &pb.ClientMessage_Stance{Stance: stance(110)}})
	far.say(&pb.ClientMessage{Message: &pb.ClientMessage_Stance{Stance: stance(1100)}})
	// The stances have to land before a line is measured against them: the
	// neighbour waits to hear both its own and the speaker's.
	landed := map[uint32]bool{}
	b.hearUntil(func(m *pb.ServerMessage) bool {
		for _, moved := range m.GetStances().GetMoved() {
			landed[moved.Session] = true
		}
		return len(landed) >= 2
	})

	said := func(c *client) *pb.Said {
		return c.hearUntil(func(m *pb.ServerMessage) bool { return m.GetSaid() != nil }).GetSaid()
	}
	a.say(&pb.ClientMessage{Message: &pb.ClientMessage_Say{Say: &pb.Say{Scope: pb.Scope_SCOPE_NEAR, Text: "  hi  ", Here: true}}})
	for _, c := range []*client{a, b} {
		line := said(c)
		if line.Session != me || line.Text != "hi" || line.Scope != pb.Scope_SCOPE_NEAR {
			t.Fatalf("a near line reaches the speaker and a neighbour, trimmed: %v", line)
		}
		if line.Stance == nil || line.Stance.U != 100 {
			t.Fatalf("with the speaker's place, as the actor saw it: %v", line)
		}
	}
	a.say(&pb.ClientMessage{Message: &pb.ClientMessage_Say{Say: &pb.Say{Scope: pb.Scope_SCOPE_WORLD, Text: "all"}}})
	if line := said(far); line.Text != "all" || line.Stance != nil {
		t.Fatalf("the far one hears only the world line, and no place unasked: %v", line)
	}

	long := strings.Repeat("x", LineChars+1)
	a.say(&pb.ClientMessage{Message: &pb.ClientMessage_Say{Say: &pb.Say{Scope: pb.Scope_SCOPE_WORLD, Text: long}}})
	a.say(&pb.ClientMessage{Message: &pb.ClientMessage_Say{Say: &pb.Say{Scope: pb.Scope_SCOPE_WORLD}}})
	// Characters, not bytes: a full line of accents is still a line.
	accented := strings.Repeat("ç", LineChars)
	a.say(&pb.ClientMessage{Message: &pb.ClientMessage_Say{Say: &pb.Say{Scope: pb.Scope_SCOPE_WORLD, Text: accented}}})
	for i := range lineBurst + 2 {
		a.say(&pb.ClientMessage{Message: &pb.ClientMessage_Say{Say: &pb.Say{Scope: pb.Scope_SCOPE_WORLD, Text: strconv.Itoa(i)}}})
	}
	a.say(&pb.ClientMessage{Message: &pb.ClientMessage_Wear{Wear: &pb.Wear{Avatar: "avatars/Ada.vrm"}}})
	var heard []string
	far.hearUntil(func(m *pb.ServerMessage) bool {
		if line := m.GetSaid(); line != nil {
			heard = append(heard, line.Text)
		}
		return m.GetWearing() != nil
	})
	// Two lines were already said in this window: three more fit.
	if want := []string{accented, "0", "1"}; !slices.Equal(heard, want) {
		t.Fatalf("too long, empty and past the rate are dropped: %v", heard)
	}

	a.leave()
	b.leave()
	far.leave()
}
