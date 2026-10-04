package module

import (
	"context"
	"math"
	"testing"
	"unicode/utf8"

	"google.golang.org/protobuf/proto"

	pb "github.com/sulram/planet/server/internal/protocol"
	bridge "github.com/sulram/planet/server/internal/protocol/module"
)

// The real module, handed what nobody means to hand it. `go test` runs the
// seeds; `go test -fuzz <name>` goes looking for more.

// Whatever bytes are run as a call, the module answers or says nothing: it
// never traps, and it is the same instance after.
func FuzzAnyBytesAsACall(f *testing.F) {
	m, err := Load(context.Background())
	if err != nil {
		f.Fatal(err)
	}
	f.Cleanup(func() { m.Close(context.Background()) })
	for _, call := range []*bridge.Call{
		describe,
		{Call: &bridge.Call_Start{Start: &bridge.Start{Recipe: &pb.Recipe{Seed: "00000000deadbeef", GeneratorVersion: 3, ParamsJson: "{}"}}}},
		{Call: &bridge.Call_Op{Op: &bridge.Op{Plugin: "chat", Kind: "say", Payload: say(scopeWorld, "hi"), Session: 1, Here: []*bridge.Who{{Session: 1}}}}},
		{Call: &bridge.Call_Gone{Gone: &bridge.Gone{Plugin: "chat", Session: 1}}},
	} {
		frame, _ := proto.Marshal(call)
		f.Add(frame)
	}
	f.Add([]byte{})
	f.Add([]byte{0xff, 0xff, 0xff, 0xff})
	f.Fuzz(func(t *testing.T, call []byte) {
		if _, err := raw(m, call); err != nil {
			t.Fatalf("the module faulted: %v", err)
		}
		if m.born != 1 {
			t.Fatalf("the instance was replaced: born %d", m.born)
		}
	})
}

// An op of any shape, from a body standing anywhere, numbers that are no
// number among them: the module never traps, and what it tells is chat's own,
// for those in the room alone.
func FuzzAnOpOfAnyShape(f *testing.F) {
	m, err := Load(context.Background())
	if err != nil {
		f.Fatal(err)
	}
	f.Cleanup(func() { m.Close(context.Background()) })
	nan, inf := float32(math.NaN()), float32(math.Inf(1))
	f.Add(say(scopeNear, "hi"), "say", float32(32768), float32(32768), float32(0), uint32(2), int32(0), uint64(1_790_000_000_000))
	f.Add(say(scopeNear, "hi"), "say", nan, nan, nan, uint32(2), int32(0), uint64(0))
	f.Add(say(scopeNear, "hi"), "say", inf, -inf, inf, uint32(7), int32(1), uint64(math.MaxUint64))
	f.Add([]byte{0xff}, "shout", float32(-1), float32(1e30), float32(-1e30), uint32(math.MaxUint32), int32(-1), uint64(1))
	f.Fuzz(func(t *testing.T, payload []byte, kind string, u, v, height float32, sector uint32, body int32, now uint64) {
		// A kind is a string of the wire, and the wire carries valid UTF-8
		// alone: a frame with any other is refused before it is a session's.
		if !utf8.ValidString(kind) {
			t.Skip()
		}
		start := &bridge.Call{Call: &bridge.Call_Start{Start: &bridge.Start{Recipe: &pb.Recipe{Seed: "00000000deadbeef", GeneratorVersion: 3, ParamsJson: "{}"}}}}
		if _, err := m.call(context.Background(), start); err != nil {
			t.Fatalf("the world did not start: %v", err)
		}
		asks := &pb.Stance{Body: pb.Body(body), Sector: sector, U: u, V: v, HeightM: height}
		op := &bridge.Op{Plugin: "chat", Kind: kind, Payload: payload, Session: 1, NowMs: now, Here: []*bridge.Who{
			{Session: 1, Stance: asks},
			{Session: 2, Stance: &pb.Stance{Sector: 2, U: 32768, V: 32768}},
			{Session: 3},
		}}
		replies, err := m.call(context.Background(), &bridge.Call{Call: &bridge.Call_Op{Op: op}})
		if err != nil {
			t.Fatalf("the module faulted: %v", err)
		}
		for _, reply := range replies {
			tell := reply.GetTell()
			if tell == nil || tell.Plugin != "chat" || tell.Kind != "said" {
				t.Fatalf("chat says one thing, under its own name: %v", reply)
			}
			for _, session := range tell.To {
				if session < 1 || session > 3 {
					t.Fatalf("told to someone who is not here: %v", tell.To)
				}
			}
		}
		if m.born != 1 {
			t.Fatalf("the instance was replaced: born %d", m.born)
		}
	})
}
