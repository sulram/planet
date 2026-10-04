package module_test

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/coder/websocket"
	"google.golang.org/protobuf/encoding/protowire"
	"google.golang.org/protobuf/proto"

	"github.com/sulram/planet/server/internal/api"
	"github.com/sulram/planet/server/internal/folder"
	"github.com/sulram/planet/server/internal/module"
	pb "github.com/sulram/planet/server/internal/protocol"
	"github.com/sulram/planet/server/internal/world"
)

// A whole instance over the module this binary carries: the routes, the
// socket, the world folder and the world halves, as `planet serve` wires
// them.

// instance is a world alone over a folder, with the plugins the module
// carries, where every session has `level`.
func instance(t *testing.T, dir string, level world.Level) *httptest.Server {
	t.Helper()
	kept, err := folder.Open(dir)
	if err != nil {
		t.Fatal(err)
	}
	hosted, err := module.Load(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { hosted.Close(context.Background()) })
	web := httptest.NewServer(api.New(api.Config{DevLevel: level, Version: "test"}, kept, hosted.Plugins()).Handler())
	t.Cleanup(web.Close)
	return web
}

func call(t *testing.T, method, url string, body any) (int, map[string]any) {
	t.Helper()
	raw, _ := json.Marshal(body)
	request, _ := http.NewRequest(method, url, bytes.NewReader(raw))
	response, err := http.DefaultClient.Do(request)
	if err != nil {
		t.Fatal(err)
	}
	defer response.Body.Close()
	var answer map[string]any
	_ = json.NewDecoder(response.Body).Decode(&answer)
	return response.StatusCode, answer
}

// socket is a client on a world's socket, past its Hello.
type socket struct {
	t    *testing.T
	conn *websocket.Conn
}

func dial(t *testing.T, web *httptest.Server) (*socket, *pb.Welcome) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	t.Cleanup(cancel)
	conn, _, err := websocket.Dial(ctx, "ws"+strings.TrimPrefix(web.URL, "http")+"/api/socket", nil)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { conn.CloseNow() })
	s := &socket{t: t, conn: conn}
	s.say(&pb.ClientMessage{Message: &pb.ClientMessage_Hello{Hello: &pb.Hello{Protocol: world.Protocol}}})
	welcome := s.hear(func(m *pb.ServerMessage) bool { return m.GetWelcome() != nil }).GetWelcome()
	return s, welcome
}

func (s *socket) say(message *pb.ClientMessage) {
	s.t.Helper()
	frame, err := proto.Marshal(message)
	if err != nil {
		s.t.Fatal(err)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	if err := s.conn.Write(ctx, websocket.MessageBinary, frame); err != nil {
		s.t.Fatal(err)
	}
}

// hear skips heartbeats and anything else until `want` matches.
func (s *socket) hear(want func(*pb.ServerMessage) bool) *pb.ServerMessage {
	s.t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	for {
		_, frame, err := s.conn.Read(ctx)
		if err != nil {
			s.t.Fatalf("never heard what was wanted: %v", err)
		}
		var message pb.ServerMessage
		if err := proto.Unmarshal(frame, &message); err != nil {
			s.t.Fatal(err)
		}
		if want(&message) {
			return &message
		}
	}
}

// line is a line of chat for the whole world, in an envelope. Chat's wire is
// written by its numbers: the server has no schema of a plugin.
func line(text string) *pb.ClientMessage {
	var say []byte
	say = protowire.AppendTag(say, 1, protowire.VarintType)
	say = protowire.AppendVarint(say, 1)
	say = protowire.AppendTag(say, 2, protowire.BytesType)
	say = protowire.AppendString(say, text)
	return &pb.ClientMessage{Message: &pb.ClientMessage_Envelope{Envelope: &pb.Envelope{Plugin: "chat", Kind: "say", Payload: say}}}
}

// said reads the session and the text of a line heard, its first and third
// fields.
func said(t *testing.T, payload []byte) (session uint64, text string) {
	t.Helper()
	for len(payload) > 0 {
		number, kind, n := protowire.ConsumeTag(payload)
		if n < 0 {
			t.Fatal("a said that is no message")
		}
		payload = payload[n:]
		switch {
		case number == 1 && kind == protowire.VarintType:
			session, _ = protowire.ConsumeVarint(payload)
		case number == 3 && kind == protowire.BytesType:
			text, _ = protowire.ConsumeString(payload)
		}
		payload = payload[protowire.ConsumeFieldValue(number, kind, payload):]
	}
	return session, text
}

var recipe = map[string]any{"seed": "00000000deadbeef", "generator_version": 3, "params": map[string]any{}}

func spoken(about map[string]any) []string {
	var names []string
	on, _ := about["plugins"].([]any)
	for _, p := range on {
		names = append(names, p.(map[string]any)["name"].(string))
	}
	return names
}

func TestChatRidesTheEnvelopeAndAnAdminSwitchesIt(t *testing.T) {
	dir := t.TempDir()
	web := instance(t, dir, world.LevelAdmin)
	if status, _ := call(t, "POST", web.URL+"/api/world", recipe); status != http.StatusOK {
		t.Fatalf("the world is founded: %d", status)
	}
	if _, about := call(t, "GET", web.URL+"/api/world", nil); len(spoken(about)) != 1 || spoken(about)[0] != "chat" {
		t.Fatalf("a world says which plugins it speaks: %v", about)
	}

	a, welcome := dial(t, web)
	if len(welcome.Plugins) != 1 || welcome.Plugins[0].Name != "chat" || welcome.Plugins[0].Version != 1 {
		t.Fatalf("and says it again in its welcome: %v", welcome.Plugins)
	}
	a.say(line("hi"))
	heard := a.hear(func(m *pb.ServerMessage) bool { return m.GetEnvelope() != nil }).GetEnvelope()
	session, text := said(t, heard.Payload)
	if heard.Plugin != "chat" || heard.Kind != "said" || text != "hi" || uint32(session) != welcome.Session {
		t.Fatalf("a line goes up in an envelope and comes back in one: %v %d %q", heard, session, text)
	}

	if status, answer := call(t, "POST", web.URL+"/api/plugins", map[string]any{"name": "voxels", "on": true}); status != http.StatusNotFound || answer["error"] != "plugin" {
		t.Fatalf("a plugin the version lacks is none to switch: %d %v", status, answer)
	}
	if status, answer := call(t, "POST", web.URL+"/api/plugins", map[string]any{"name": "chat"}); status != http.StatusBadRequest || answer["error"] != "body" {
		t.Fatalf("on or off is said: %d %v", status, answer)
	}
	status, about := call(t, "POST", web.URL+"/api/plugins", map[string]any{"name": "chat", "on": false})
	if status != http.StatusOK || len(spoken(about)) != 0 {
		t.Fatalf("an admin switches chat off: %d %v", status, about)
	}
	if on := a.hear(func(m *pb.ServerMessage) bool { return m.GetPlugins() != nil }).GetPlugins(); len(on.Plugins) != 0 {
		t.Fatalf("whoever is here is told: %v", on)
	}
	// Off, a line is dropped: the next thing heard from a plugin is none.
	a.say(line("anyone?"))
	b, late := dial(t, web)
	if len(late.Plugins) != 0 {
		t.Fatalf("and whoever arrives after hears it off: %v", late.Plugins)
	}
	b.say(&pb.ClientMessage{Message: &pb.ClientMessage_Wear{Wear: &pb.Wear{Avatar: "avatars/Ada.vrm"}}})
	if m := a.hear(func(m *pb.ServerMessage) bool { return m.GetEnvelope() != nil || m.GetWearing() != nil }); m.GetEnvelope() != nil {
		t.Fatalf("a plugin that is off says nothing: %v", m)
	}

	// The choice is the world's own: it is in the folder, and a new process
	// over the same folder reads it.
	again := instance(t, dir, world.LevelAdmin)
	if _, about := call(t, "GET", again.URL+"/api/world", nil); len(spoken(about)) != 0 {
		t.Fatalf("the folder keeps the admin's word: %v", about)
	}
	if status, about := call(t, "POST", again.URL+"/api/plugins", map[string]any{"name": "chat", "on": true}); status != http.StatusOK || len(spoken(about)) != 1 {
		t.Fatalf("and it is switched back: %d %v", status, about)
	}
}

func TestOnlyAnAdminSwitchesAPlugin(t *testing.T) {
	web := instance(t, t.TempDir(), world.LevelBuilder)
	status, answer := call(t, "POST", web.URL+"/api/plugins", map[string]any{"name": "chat", "on": false})
	if status != http.StatusForbidden || answer["error"] != "level" {
		t.Fatalf("a builder switches nothing: %d %v", status, answer)
	}
	if _, about := call(t, "GET", web.URL+"/api/world", nil); len(spoken(about)) != 1 {
		t.Fatalf("and chat stays as the config says: %v", about)
	}
}
