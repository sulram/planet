package module

import (
	"context"
	"runtime"
	"testing"
	"time"

	"google.golang.org/protobuf/proto"

	bridge "github.com/sulram/planet/server/internal/protocol/module"
	"github.com/sulram/planet/server/internal/world"
)

// Modules written by hand to break the bridge from the inside: what a world
// half gone wrong, or one written to harm, could do with the four names.

func leb(n uint32) []byte {
	var out []byte
	for {
		b := byte(n & 0x7f)
		n >>= 7
		if n == 0 {
			return append(out, b)
		}
		out = append(out, b|0x80)
	}
}

// i32 is `i32.const n`.
func i32(n int32) []byte {
	out := []byte{0x41}
	for {
		b := byte(n & 0x7f)
		n >>= 7
		if (n == 0 && b&0x40 == 0) || (n == -1 && b&0x40 != 0) {
			return append(out, b)
		}
		out = append(out, b|0x80)
	}
}

func join(parts ...[]byte) []byte {
	var out []byte
	for _, part := range parts {
		out = append(out, part...)
	}
	return out
}

func section(id byte, content []byte) []byte {
	return join([]byte{id}, leb(uint32(len(content))), content)
}

func named(name string) []byte { return join(leb(uint32(len(name))), []byte(name)) }

func body(code []byte) []byte {
	code = join([]byte{0x00}, code, []byte{0x0b}) // no locals, the code, end
	return join(leb(uint32(len(code))), code)
}

// The imports `host.reply` and `host.ask` are functions 0 and 1, `reserve` 2
// and `call` 3.
var (
	sayReply = []byte{0x10, 0x00}
	// A question, and its answer's length dropped.
	askStore = []byte{0x10, 0x01, 0x1a}
)

// hostile is a module with `pages` of memory, `data` at address zero, and the
// two exports as the bodies given.
func hostile(pages uint32, data, reserve, call []byte) []byte {
	module := join(
		[]byte{0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00},
		// (i32) -> i32, () -> (), (i32, i32) -> (), (i32, i32) -> i32
		section(1, []byte{0x04, 0x60, 0x01, 0x7f, 0x01, 0x7f, 0x60, 0x00, 0x00, 0x60, 0x02, 0x7f, 0x7f, 0x00, 0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f}),
		section(2, join([]byte{0x02},
			named("host"), named("reply"), []byte{0x00, 0x02},
			named("host"), named("ask"), []byte{0x00, 0x03})),
		section(3, []byte{0x02, 0x00, 0x01}),
		section(5, join([]byte{0x01, 0x00}, leb(pages))),
		section(7, join([]byte{0x03},
			named("memory"), []byte{0x02, 0x00},
			named("reserve"), []byte{0x00, 0x02},
			named("call"), []byte{0x00, 0x03})),
		section(10, join([]byte{0x02}, body(reserve), body(call))),
	)
	if len(data) > 0 {
		module = append(module, section(11, join([]byte{0x01, 0x00}, i32(0), []byte{0x0b}, leb(uint32(len(data))), data))...)
	}
	return module
}

var describe = &bridge.Call{Call: &bridge.Call_Describe{Describe: &bridge.Describe{}}}

func opened(t *testing.T, wasm []byte) *Module {
	t.Helper()
	m, err := open(context.Background(), wasm)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { m.Close(context.Background()) })
	return m
}

// heap is how much this process holds, after a collection.
func heap() uint64 {
	runtime.GC()
	var stats runtime.MemStats
	runtime.ReadMemStats(&stats)
	return stats.HeapAlloc
}

func TestAnAddressOutsideTheModuleIsRefused(t *testing.T) {
	outside := i32(-16) // 0xfffffff0, far past a page of memory
	for name, wasm := range map[string][]byte{
		"a reply that points outside the module": hostile(1, nil, i32(0), join(outside, i32(64), sayReply)),
		"a reply longer than the memory":         hostile(1, nil, i32(0), join(i32(65000), i32(4096), sayReply)),
		"an inbox that points outside":           hostile(1, nil, outside, nil),
	} {
		m := opened(t, wasm)
		if _, err := m.call(context.Background(), describe); err == nil {
			t.Fatalf("%s: the call is a fault", name)
		}
		if m.born != 2 {
			t.Fatalf("%s: and the instance is replaced: born %d", name, m.born)
		}
	}
}

func TestAQuestionTheServerCannotTakeIsAFault(t *testing.T) {
	outside := i32(-16)
	// Ask{owner: "other", get: "k"}, asked while no owner's call runs.
	another := []byte{0x0a, 0x05, 'o', 't', 'h', 'e', 'r', 0x12, 0x01, 'k'}
	for name, wasm := range map[string][]byte{
		"a question that points outside the module": hostile(1, nil, i32(0), join(outside, i32(16), askStore)),
		"a question longer than one may be":         hostile(4, nil, i32(0), join(i32(0), i32(maxAskSize+1), askStore)),
		"a question of another owner's store":       hostile(1, another, i32(0), join(i32(0), i32(int32(len(another))), askStore)),
		"an answer with no inbox to be written in":  hostile(1, nil, outside, join(i32(0), i32(0), askStore)),
		"questions without end":                     hostile(1, nil, i32(0), join([]byte{0x03, 0x40}, i32(0), i32(0), askStore, []byte{0x0c, 0x00, 0x0b})),
	} {
		m := opened(t, wasm)
		began := time.Now()
		if _, err := m.call(context.Background(), describe); err == nil {
			t.Fatalf("%s: the call is a fault", name)
		}
		if took := time.Since(began); took > deadline/2 {
			t.Fatalf("%s: it is stopped long before the deadline: %v", name, took)
		}
		if m.born != 2 {
			t.Fatalf("%s: and the instance is replaced: born %d", name, m.born)
		}
	}
	// A question the server can take is answered, with nothing where
	// nothing is kept.
	m := opened(t, hostile(1, nil, i32(0), join(i32(0), i32(0), askStore)))
	if _, err := m.call(context.Background(), describe); err != nil || m.born != 1 {
		t.Fatalf("an empty question is one: %v, born %d", err, m.born)
	}
}

func TestAModuleThatRepliesWithoutEndIsStopped(t *testing.T) {
	// An empty reply is a message, so each one is kept: a loop of them is
	// memory taken on the server's side of the bridge, outside the module's
	// ceiling.
	flood := hostile(1, nil, i32(0), join([]byte{0x03, 0x40}, i32(0), i32(0), sayReply, []byte{0x0c, 0x00, 0x0b}))
	m := opened(t, flood)
	before := heap()
	began := time.Now()
	if _, err := m.call(context.Background(), describe); err == nil {
		t.Fatal("the call is a fault")
	}
	took, grew := time.Since(began), int64(heap())-int64(before)
	if took > deadline/2 {
		t.Fatalf("it is stopped at the limit of replies, long before the deadline: %v", took)
	}
	if grew > 8<<20 {
		t.Fatalf("and the server holds none of the flood: %d bytes", grew)
	}
}

func TestAReplyLargerThanACallMaySayIsRefused(t *testing.T) {
	// Sixty four pages of memory, 4 MB, handed over as one reply.
	huge := hostile(64, nil, i32(0), join(i32(0), i32(4<<20), sayReply))
	m := opened(t, huge)
	if _, err := m.call(context.Background(), describe); err == nil {
		t.Fatal("the call is a fault")
	}
}

func TestAModuleTakesNoMoreMemoryThanItsCeiling(t *testing.T) {
	// Grows a page at a time until it is refused, then returns.
	hog := hostile(1, nil, i32(0), join([]byte{0x03, 0x40}, i32(1), []byte{0x40, 0x00}, i32(-1), []byte{0x47, 0x0d, 0x00, 0x0b}))
	m := opened(t, hog)
	if err := m.ask(context.Background(), nil); err != nil {
		t.Fatalf("it returns once it is refused: %v", err)
	}
	if size := m.instance.Memory().Size(); size != memoryPages*65536 {
		t.Fatalf("it stops at the ceiling: %d bytes", size)
	}
}

func TestAWorldHalfSpeaksUnderItsOwnNameAlone(t *testing.T) {
	// A module that answers every call with an event under another plugin's
	// name, for everyone and for a session that is not here.
	lie, _ := proto.Marshal(&bridge.Reply{Reply: &bridge.Reply_Tell{Tell: &bridge.Tell{
		Plugin: "land", Kind: "granted", Payload: []byte("everything"), To: []uint32{speaker, neighbour, far, 99},
	}}})
	m := opened(t, hostile(1, lie, i32(4096), join(i32(0), i32(int32(len(lie))), sayReply)))
	r, who := three()
	(&hosted{module: m, name: "chat"}).Do(r, who, "say", say(scopeWorld, "hi"))
	if len(r.heard) != 0 {
		t.Fatalf("chat's op tells nothing as land: %v", r.heard)
	}
	// Under its own name it is told, and a session that is not here hears nothing.
	(&hosted{module: m, name: "land"}).Do(r, who, "grant", nil)
	if len(r.heard) != 3 || len(r.heard[99]) != 0 {
		t.Fatalf("to those in the room alone: %v", r.heard)
	}
}

func TestAModuleWithoutTheNamesIsNoModule(t *testing.T) {
	// The two types and nothing else: no `reserve`, no `call`.
	empty := []byte{0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00}
	if _, err := open(context.Background(), empty); err == nil {
		t.Fatal("a module that exports neither name is refused when it is opened")
	}
}

// The real module, asked in ways the server never asks.
func TestTheRealModuleAskedWrong(t *testing.T) {
	m := loaded(t)
	ctx := context.Background()
	reserve, call := m.instance.ExportedFunction("reserve"), m.instance.ExportedFunction("call")

	// Nothing reserved, and an inbox of nothing: no reply, no fault.
	if _, err := call.Call(ctx); err != nil {
		t.Fatalf("a call with nothing in the inbox: %v", err)
	}
	if replies, err := raw(m, nil); err != nil || len(replies) != 0 {
		t.Fatalf("an empty call says nothing: %v %v", replies, err)
	}
	// A call served is gone from the inbox: run again, it says nothing twice.
	frame, _ := proto.Marshal(describe)
	if replies, err := raw(m, frame); err != nil || len(replies) != 1 {
		t.Fatalf("it says what it carries: %v %v", replies, err)
	}
	m.replies = nil
	if _, err := call.Call(ctx); err != nil || len(m.replies) != 0 {
		t.Fatalf("a served call is not served again: %v %v", m.replies, err)
	}
	// An inbox larger than the ceiling is a trap, and never a write.
	if _, err := reserve.Call(ctx, 0xffff_fff0); err == nil {
		t.Fatal("four gigabytes of inbox is refused")
	}
	// The instance that trapped is replaced, and the next one hosts chat.
	m.mu.Lock()
	if err := m.replace(ctx); err != nil {
		t.Fatal(err)
	}
	m.mu.Unlock()
	r, who := three()
	chat(t, m).Do(r, who, "say", say(scopeWorld, "still here"))
	if len(r.heard[far]) != 1 {
		t.Fatalf("after all of it a line is still heard: %v", r.heard)
	}
}

// raw runs bytes as a call, as they are, and gives what the module said.
func raw(m *Module, frame []byte) ([]*bridge.Reply, error) {
	m.replies, m.fault = nil, nil
	err := m.ask(context.Background(), frame)
	return m.replies, err
}

var _ = world.LevelAnonymous
