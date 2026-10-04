package chat

import (
	"slices"
	"strconv"
	"strings"
	"testing"
	"time"

	"google.golang.org/protobuf/proto"

	"github.com/sulram/planet/server/internal/chat/wire"
	pb "github.com/sulram/planet/server/internal/protocol"
	"github.com/sulram/planet/server/internal/world"
)

// room is a world of three as chat sees it: the speaker, a neighbour and
// someone far away. It keeps what each one heard.
type room struct {
	t     *testing.T
	now   time.Time
	here  []world.Who
	near  map[[2]uint32]bool
	heard map[uint32][]*wire.Said
}

func (r *room) Now() time.Time { return r.now }

func (r *room) Tell(kind string, payload []byte, to func(world.Who) bool) {
	if kind != "said" {
		r.t.Fatalf("chat says %q", kind)
	}
	var said wire.Said
	if err := proto.Unmarshal(payload, &said); err != nil {
		r.t.Fatal(err)
	}
	for _, who := range r.here {
		if to(who) {
			r.heard[who.Session] = append(r.heard[who.Session], &said)
		}
	}
}

func (r *room) Near(a, b world.Who, blocks float64) bool {
	if blocks != NearBlocks {
		r.t.Fatalf("near is asked about %v blocks", blocks)
	}
	return r.near[[2]uint32{a.Session, b.Session}]
}

func (r *room) texts(session uint32) []string {
	var texts []string
	for _, said := range r.heard[session] {
		texts = append(texts, said.Text)
	}
	return texts
}

const speaker, neighbour, far = 1, 2, 3

func three(t *testing.T) (*room, world.Who) {
	who := world.Who{Session: speaker, Stance: &pb.Stance{Sector: 2, U: 100, V: 100}}
	return &room{
		t:     t,
		now:   time.Unix(1790000000, 0),
		here:  []world.Who{who, {Session: neighbour}, {Session: far}},
		near:  map[[2]uint32]bool{{speaker, neighbour}: true},
		heard: map[uint32][]*wire.Said{},
	}, who
}

func say(t *testing.T, c *Chat, r *room, who world.Who, line *wire.Say) {
	t.Helper()
	payload, err := proto.Marshal(line)
	if err != nil {
		t.Fatal(err)
	}
	c.Do(r, who, "say", payload)
}

func TestAnyoneMaySpeak(t *testing.T) {
	ops := New().Ops()
	if len(ops) != 1 || ops[0] != (world.Op{Kind: "say", Level: world.LevelAnonymous}) {
		t.Fatalf("chat offers one op, to every level: %v", ops)
	}
}

func TestALineReachesItsScope(t *testing.T) {
	c := New()
	r, who := three(t)

	say(t, c, r, who, &wire.Say{Scope: wire.Scope_SCOPE_NEAR, Text: "  hi  ", Here: true})
	for _, session := range []uint32{speaker, neighbour} {
		heard := r.heard[session]
		if len(heard) != 1 || heard[0].Session != speaker || heard[0].Text != "hi" || heard[0].Scope != wire.Scope_SCOPE_NEAR {
			t.Fatalf("a near line reaches the speaker and a neighbour, trimmed: %v", heard)
		}
		if heard[0].Stance == nil || heard[0].Stance.U != 100 {
			t.Fatalf("with the speaker's place, as the actor saw it: %v", heard[0])
		}
	}
	if len(r.heard[far]) != 0 {
		t.Fatalf("the far one hears no near line: %v", r.texts(far))
	}

	say(t, c, r, who, &wire.Say{Scope: wire.Scope_SCOPE_WORLD, Text: "all"})
	if heard := r.heard[far]; len(heard) != 1 || heard[0].Text != "all" || heard[0].Stance != nil {
		t.Fatalf("the far one hears the world line, and no place unasked: %v", heard)
	}
}

func TestWhatIsTooLongEmptyOrTooFastIsDropped(t *testing.T) {
	c := New()
	r, who := three(t)
	world := func(text string) *wire.Say { return &wire.Say{Scope: wire.Scope_SCOPE_WORLD, Text: text} }

	say(t, c, r, who, world(strings.Repeat("x", LineChars+1)))
	say(t, c, r, who, world(""))
	c.Do(r, who, "say", []byte{0xff, 0xff})
	c.Do(r, who, "shout", nil)
	// Characters, not bytes: a full line of accents is still a line.
	accented := strings.Repeat("ç", LineChars)
	say(t, c, r, who, world(accented))
	for i := range lineBurst + 2 {
		say(t, c, r, who, world(strconv.Itoa(i)))
	}
	if want := []string{accented, "0", "1", "2", "3"}; !slices.Equal(r.texts(far), want) {
		t.Fatalf("too long, empty, unread and past the rate are dropped: %v", r.texts(far))
	}

	// The window passes, and a session that left is counted from nothing.
	r.now = r.now.Add(lineWindow)
	say(t, c, r, who, world("later"))
	c.Gone(speaker)
	for i := range lineBurst {
		say(t, c, r, who, world("again "+strconv.Itoa(i)))
	}
	if heard := r.texts(far); len(heard) != 5+1+lineBurst || heard[5] != "later" {
		t.Fatalf("the rate is a window, kept while a session is here: %v", heard)
	}
}
