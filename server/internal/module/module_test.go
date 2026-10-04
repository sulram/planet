package module

import (
	"context"
	"slices"
	"strings"
	"testing"
	"time"

	"google.golang.org/protobuf/encoding/protowire"

	pb "github.com/sulram/planet/server/internal/protocol"
	bridge "github.com/sulram/planet/server/internal/protocol/module"
	"github.com/sulram/planet/server/internal/world"
)

func loaded(t *testing.T) *Module {
	t.Helper()
	m, err := Load(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { m.Close(context.Background()) })
	return m
}

// chat is the one plugin this version carries, as the core hosts it.
func chat(t *testing.T, m *Module) world.Plugin {
	t.Helper()
	for _, in := range m.Plugins() {
		if in.Plugin.Name() == "chat" {
			return in.Plugin
		}
	}
	t.Fatal("the module carries no chat")
	return nil
}

func TestTheModuleSaysWhatItCarries(t *testing.T) {
	plugins := loaded(t).Plugins()
	if len(plugins) != 1 || plugins[0].Plugin.Name() != "chat" || plugins[0].Plugin.Version() != 1 || !plugins[0].On {
		t.Fatalf("this version carries chat, on: %v", plugins)
	}
	if ops := plugins[0].Plugin.Ops(); len(ops) != 1 || ops[0] != (world.Op{Kind: "say", Level: world.LevelAnonymous}) {
		t.Fatalf("chat offers one op, to every level: %v", ops)
	}
}

// Chat's wire, written by its numbers: the server has no schema of a plugin.
const (
	scopeNear  = 0
	scopeWorld = 1
)

func say(scope uint64, text string) []byte {
	var line []byte
	line = protowire.AppendTag(line, 1, protowire.VarintType)
	line = protowire.AppendVarint(line, scope)
	line = protowire.AppendTag(line, 2, protowire.BytesType)
	return protowire.AppendString(line, text)
}

// text is the line a Said carries, its third field.
func text(t *testing.T, said []byte) string {
	t.Helper()
	for len(said) > 0 {
		number, kind, n := protowire.ConsumeTag(said)
		if n < 0 {
			t.Fatal("a said that is no message")
		}
		said = said[n:]
		if number == 3 && kind == protowire.BytesType {
			value, _ := protowire.ConsumeString(said)
			return value
		}
		said = said[protowire.ConsumeFieldValue(number, kind, said):]
	}
	return ""
}

// room is a world as a plugin sees it, keeping what each session heard.
type room struct {
	now   time.Time
	here  []world.Who
	heard map[uint32][]string
}

func (r *room) Now() time.Time { return r.now }
func (r *room) Recipe() world.Recipe {
	recipe, _ := world.NewRecipe("00000000deadbeef", 3, nil)
	return recipe
}
func (r *room) Here() []world.Who { return r.here }
func (r *room) Tell(kind string, payload []byte, to func(world.Who) bool) {
	for _, who := range r.here {
		if to(who) {
			r.heard[who.Session] = append(r.heard[who.Session], kind+":"+string(payload))
		}
	}
}

const speaker, neighbour, far = 1, 2, 3

// three is the speaker, a neighbour sixty blocks along and someone seventy
// blocks along: one inside chat's reach, one outside it.
func three() (*room, world.Who) {
	at := func(session uint32, u float32) world.Who {
		return world.Who{Session: session, Stance: &pb.Stance{Sector: 2, U: u, V: 32768}}
	}
	who := at(speaker, 32768)
	return &room{
		now:   time.Unix(1790000000, 0),
		here:  []world.Who{who, at(neighbour, 32768+60), at(far, 32768+70)},
		heard: map[uint32][]string{},
	}, who
}

func TestALineCrossesTheBridgeAndReachesItsScope(t *testing.T) {
	m := loaded(t)
	plugin := chat(t, m)
	r, who := three()

	plugin.Do(r, who, "say", say(scopeNear, "hi"))
	for _, session := range []uint32{speaker, neighbour} {
		if heard := r.heard[session]; len(heard) != 1 || !strings.HasPrefix(heard[0], "said:") {
			t.Fatalf("a near line reaches the speaker and a neighbour: %v", r.heard)
		}
	}
	if len(r.heard[far]) != 0 {
		t.Fatalf("the far one hears no near line: the world half measures by the recipe: %v", r.heard[far])
	}
	plugin.Do(r, who, "say", say(scopeWorld, "all"))
	if heard := r.heard[far]; len(heard) != 1 {
		t.Fatalf("the far one hears the world line: %v", heard)
	}

	// The world half keeps its count between calls: the sixth line in a
	// burst is dropped, and a session that left is counted from nothing.
	for range 6 {
		plugin.Do(r, who, "say", say(scopeWorld, "again"))
	}
	if heard := len(r.heard[far]); heard != 1+3 {
		t.Fatalf("five lines in five seconds, the two above among them: %d", heard)
	}
	plugin.Gone(speaker)
	plugin.Do(r, who, "say", say(scopeWorld, "back"))
	if heard := len(r.heard[far]); heard != 1+3+1 {
		t.Fatalf("a session that left is counted from nothing: %d", heard)
	}
	if m.born != 1 {
		t.Fatalf("and all of it in one instance: %d", m.born)
	}
}

// A module of two functions and a page of memory, written by hand: `reserve`
// answers address zero, and `call` is the body given.
func faulty(call ...byte) []byte {
	wasm := []byte{
		0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // \0asm, version 1
		0x01, 0x09, 0x02, 0x60, 0x01, 0x7f, 0x01, 0x7f, 0x60, 0x00, 0x00, // types: (i32) -> i32, () -> ()
		0x03, 0x03, 0x02, 0x00, 0x01, // functions: reserve, call
		0x05, 0x03, 0x01, 0x00, 0x01, // memory: one page
		0x07, 0x1b, 0x03, // exports
		0x06, 'm', 'e', 'm', 'o', 'r', 'y', 0x02, 0x00,
		0x07, 'r', 'e', 's', 'e', 'r', 'v', 'e', 0x00, 0x00,
		0x04, 'c', 'a', 'l', 'l', 0x00, 0x01,
	}
	reserve := []byte{0x04, 0x00, 0x41, 0x00, 0x0b} // i32.const 0
	body := append([]byte{byte(len(call) + 2), 0x00}, append(call, 0x0b)...)
	code := append([]byte{0x02}, append(reserve, body...)...)
	return append(wasm, append([]byte{0x0a, byte(len(code))}, code...)...)
}

func TestAFaultReplacesTheInstanceAndTheServerLives(t *testing.T) {
	describe := &bridge.Call{Call: &bridge.Call_Describe{Describe: &bridge.Describe{}}}
	for name, body := range map[string][]byte{
		"a world half that panics":        {0x00},                         // unreachable
		"a world half that never returns": {0x03, 0x40, 0x0c, 0x00, 0x0b}, // loop, br 0, end
	} {
		m, err := open(context.Background(), faulty(body...))
		if err != nil {
			t.Fatalf("%s: %v", name, err)
		}
		began := time.Now()
		if _, err := m.call(context.Background(), describe); err == nil {
			t.Fatalf("%s: the call is an error", name)
		}
		if took := time.Since(began); took > 4*deadline {
			t.Fatalf("%s: stopped within the deadline: %v", name, took)
		}
		if m.born != 2 || m.instance == nil || m.started != nil {
			t.Fatalf("%s: a new instance stands in its place, and the world starts again in it: born %d", name, m.born)
		}
		m.Close(context.Background())
	}
}

func TestAReplacedInstanceHostsTheWorldAgain(t *testing.T) {
	m := loaded(t)
	plugin := chat(t, m)
	r, who := three()
	plugin.Do(r, who, "say", say(scopeWorld, "before"))

	m.mu.Lock()
	if err := m.replace(context.Background()); err != nil {
		t.Fatal(err)
	}
	m.mu.Unlock()
	plugin.Do(r, who, "say", say(scopeWorld, "after"))
	heard := r.heard[far]
	if len(heard) != 2 || text(t, []byte(strings.TrimPrefix(heard[1], "said:"))) != "after" {
		t.Fatalf("the world is started in the new instance before its next op: %v", heard)
	}
	if !slices.Equal([]int{m.born}, []int{2}) {
		t.Fatalf("born %d", m.born)
	}
}

// What one line costs through the bridge, in a room of a hundred.
func BenchmarkALineInARoomOfAHundred(b *testing.B) {
	m, err := Load(context.Background())
	if err != nil {
		b.Fatal(err)
	}
	defer m.Close(context.Background())
	plugin := m.Plugins()[0].Plugin
	r := &room{now: time.Unix(1790000000, 0), heard: map[uint32][]string{}}
	for session := uint32(1); session <= 100; session++ {
		r.here = append(r.here, world.Who{Session: session, Stance: &pb.Stance{Sector: 2, U: 32768 + float32(session), V: 32768}})
	}
	line := say(scopeNear, "a line of chat, forty bytes or so long!!")
	for b.Loop() {
		// A second a line: the rate lets every one through.
		r.now = r.now.Add(time.Second)
		clear(r.heard)
		plugin.Do(r, r.here[0], "say", line)
	}
}
