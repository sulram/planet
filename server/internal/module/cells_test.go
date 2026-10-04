package module_test

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"google.golang.org/protobuf/proto"

	pb "github.com/sulram/planet/server/internal/protocol"
	"github.com/sulram/planet/server/internal/protocol/cells"
	"github.com/sulram/planet/server/internal/world"
)

// The world the engine's own tests build in, and dry land in the middle of
// one of its sectors.
var (
	built  = map[string]any{"seed": "0000000000000001", "generator_version": 3, "params": map[string]any{}}
	sector = uint32(4)
	atU    = 0.41 * 65536
	atV    = 0.37 * 65536
)

func seat() *cells.Seat { return &cells.Seat{Seat: &cells.Seat_Sector{Sector: sector}} }

// The height of the ground where the tests build, metres, to the nearest
// the tests need: a volume there is told and held by it.
const groundM = 300

// stands puts a body at the height of the ground, at a column `along` blocks
// from where the tests build.
func (s *socket) stands(along float64) {
	s.say(&pb.ClientMessage{Message: &pb.ClientMessage_Stance{Stance: &pb.Stance{
		Sector: sector, U: float32(atU + along), V: float32(atV), HeightM: groundM, FacingZ: -1,
	}}})
}

// asks sends an op of the cells with an id to be answered by.
func (s *socket) asks(id uint32, kind string, op proto.Message) {
	s.t.Helper()
	payload, err := proto.Marshal(op)
	if err != nil {
		s.t.Fatal(err)
	}
	s.say(&pb.ClientMessage{Message: &pb.ClientMessage_Envelope{Envelope: &pb.Envelope{Plugin: "cells", Kind: kind, Payload: payload, Id: id}}})
}

// told is the next event of the cells of one kind, read into a message.
func (s *socket) told(kind string, into proto.Message) {
	s.t.Helper()
	heard := s.hear(func(m *pb.ServerMessage) bool { return m.GetEnvelope().GetKind() == kind }).GetEnvelope()
	if heard.Plugin != "cells" {
		s.t.Fatalf("an event of the cells: %v", heard)
	}
	if err := proto.Unmarshal(heard.Payload, into); err != nil {
		s.t.Fatal(err)
	}
}

// answered is the code the op of an id was answered with.
func (s *socket) answered(id uint32) string {
	s.t.Helper()
	answer := s.hear(func(m *pb.ServerMessage) bool { return m.GetAnswer().GetId() == id }).GetAnswer()
	return answer.Code
}

func founded(t *testing.T, dir string, level world.Level) *httptest.Server {
	t.Helper()
	web := instance(t, dir, level)
	if status, _ := call(t, "POST", web.URL+"/api/world", built); status != http.StatusOK && status != http.StatusConflict && status != http.StatusForbidden {
		t.Fatalf("the world is founded: %d", status)
	}
	return web
}

func TestAChangeIsKeptToldToWhoIsNearAndShownToWhoArrives(t *testing.T) {
	dir := t.TempDir()
	web := founded(t, dir, world.LevelAdmin)
	if _, about := call(t, "GET", web.URL+"/api/world", nil); len(spoken(about)) != 2 {
		t.Fatalf("the cells are the core's, and no plugin a world speaks of: %v", about)
	}
	if status, answer := call(t, "POST", web.URL+"/api/plugins", map[string]any{"name": "cells", "on": false}); status != http.StatusNotFound {
		t.Fatalf("nobody switches the cells: %d %v", status, answer)
	}

	a, me := dial(t, web)
	a.stands(0)
	far, _ := dial(t, web)
	far.stands(4000)

	// A change where no volume stands lands nowhere, and says so.
	row := &cells.Change{Seat: seat(), Gestures: []*cells.Gesture{{Kind: cells.Kind_KIND_CREATE, Paint: 5}}}
	a.asks(1, "change", row)
	if code := a.answered(1); code != "volume" {
		t.Fatalf("no volume stands there yet: %q", code)
	}

	a.asks(2, "open", &cells.Open{Seat: seat(), U: atU, V: atV})
	var opened cells.Opened
	a.told("opened", &opened)
	if code := a.answered(2); code != "" {
		t.Fatalf("a volume is opened on dry land: %q", code)
	}
	stood := opened.Stood
	if stood.Height < 64 || stood.Version != 0 {
		t.Fatalf("it holds the ground of its plot and the air over it: %v", stood)
	}
	if middle := (float64(stood.Low) + float64(stood.Height)/2) / 2; middle < groundM-100 || middle > groundM+100 {
		t.Fatalf("the tests stand where the ground is: the volume's middle is %v m up", middle)
	}

	// A row of cells near the top of the volume.
	x, y, z := stood.PlotX*64+3, stood.PlotY*64+3, stood.Low+int32(stood.Height)-4
	row.Gestures[0].X0, row.Gestures[0].Y0, row.Gestures[0].Z0 = x, y, z
	row.Gestures[0].X1, row.Gestures[0].Y1, row.Gestures[0].Z1 = x+9, y, z
	a.asks(3, "change", row)
	var changed cells.Changed
	a.told("changed", &changed)
	if code := a.answered(3); code != "" {
		t.Fatalf("the change lands: %q", code)
	}
	if changed.Session != me.Session || len(changed.Stood) != 1 || changed.Stood[0].Version != 1 {
		t.Fatalf("whoever is near is told who changed what: %v", &changed)
	}
	// Who stands far away hears nothing of it: the next thing it hears is
	// what everyone is told.
	a.say(&pb.ClientMessage{Message: &pb.ClientMessage_Wear{Wear: &pb.Wear{Avatar: "avatars/Ada.vrm"}}})
	if m := far.hear(func(m *pb.ServerMessage) bool { return m.GetEnvelope() != nil || m.GetWearing() != nil }); m.GetEnvelope() != nil {
		t.Fatalf("a change is told to who is near, and nobody else: %v", m)
	}

	// Another process over the same folder holds what was built, and shows
	// it to whoever arrives near: a visitor, who looks and changes nothing.
	again := founded(t, dir, world.LevelAnonymous)
	b, _ := dial(t, again)
	b.stands(0)
	b.asks(1, "look", &cells.Look{})
	var seen cells.Seen
	b.told("seen", &seen)
	if len(seen.Volumes) != 1 || seen.Volumes[0].Stood.Version != 1 || len(seen.Volumes[0].Chunks) != 1 {
		t.Fatalf("what was built is kept, and seen by who arrives: %v", &seen)
	}
	if code := b.answered(1); code != "" {
		t.Fatalf("anyone looks: %q", code)
	}
	b.asks(2, "change", row)
	if code := b.answered(2); code != "level" {
		t.Fatalf("a visitor changes nothing: %q", code)
	}
	b.asks(3, "take_back", &cells.TakeBack{})
	if code := b.answered(3); code != "level" {
		t.Fatalf("and takes nothing back: %q", code)
	}
}

func TestAChangeIsTakenBackByTheOneWhoMadeIt(t *testing.T) {
	web := founded(t, t.TempDir(), world.LevelAdmin)
	a, _ := dial(t, web)
	a.stands(0)
	a.asks(1, "take_back", &cells.TakeBack{})
	if code := a.answered(1); code != "none" {
		t.Fatalf("there is nothing to take back yet: %q", code)
	}
	a.asks(2, "open", &cells.Open{Seat: seat(), U: atU, V: atV})
	var opened cells.Opened
	a.told("opened", &opened)
	stood := opened.Stood
	x, y, z := stood.PlotX*64+3, stood.PlotY*64+3, stood.Low+int32(stood.Height)-4
	cube := &cells.Gesture{Kind: cells.Kind_KIND_CREATE, Paint: 2, X0: x, Y0: y, Z0: z, X1: x, Y1: y, Z1: z}
	a.asks(3, "change", &cells.Change{Seat: seat(), Gestures: []*cells.Gesture{cube}})
	if code := a.answered(3); code != "" {
		t.Fatalf("the change lands: %q", code)
	}
	a.asks(4, "take_back", &cells.TakeBack{})
	var restored cells.Restored
	a.told("restored", &restored)
	if code := a.answered(4); code != "" || restored.Stood[0].Version != 2 {
		t.Fatalf("it is taken back, and the volume counts it: %q %v", code, &restored)
	}
	// One cell of air, packed: a run of one.
	if len(restored.Cells) != 2 || restored.Cells[0] != 1 || restored.Cells[1] != 0 {
		t.Fatalf("the cube is air again: %v", restored.Cells)
	}
	a.asks(5, "put_back", &cells.PutBack{})
	a.told("restored", &restored)
	if code := a.answered(5); code != "" || restored.Cells[1] != 3 {
		t.Fatalf("and put back: %q %v", code, restored.Cells)
	}
}
