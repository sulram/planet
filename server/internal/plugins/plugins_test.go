package plugins_test

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
	"google.golang.org/protobuf/proto"

	"github.com/sulram/planet/server/internal/api"
	"github.com/sulram/planet/server/internal/chat/wire"
	"github.com/sulram/planet/server/internal/folder"
	"github.com/sulram/planet/server/internal/plugins"
	pb "github.com/sulram/planet/server/internal/protocol"
	"github.com/sulram/planet/server/internal/world"
)

// instance is a world alone over a folder, with the plugins this version
// carries, where every session has `level`.
func instance(t *testing.T, dir string, level world.Level) *httptest.Server {
	t.Helper()
	kept, err := folder.Open(dir)
	if err != nil {
		t.Fatal(err)
	}
	web := httptest.NewServer(api.New(api.Config{DevLevel: level, Version: "test"}, kept, plugins.All()).Handler())
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

func line(t *testing.T, text string) *pb.ClientMessage {
	t.Helper()
	payload, err := proto.Marshal(&wire.Say{Scope: wire.Scope_SCOPE_WORLD, Text: text})
	if err != nil {
		t.Fatal(err)
	}
	return &pb.ClientMessage{Message: &pb.ClientMessage_Envelope{Envelope: &pb.Envelope{Plugin: "chat", Kind: "say", Payload: payload}}}
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
	a.say(line(t, "hi"))
	heard := a.hear(func(m *pb.ServerMessage) bool { return m.GetEnvelope() != nil }).GetEnvelope()
	var said wire.Said
	if err := proto.Unmarshal(heard.Payload, &said); err != nil {
		t.Fatal(err)
	}
	if heard.Plugin != "chat" || heard.Kind != "said" || said.Text != "hi" || said.Session != welcome.Session {
		t.Fatalf("a line goes up in an envelope and comes back in one: %v %v", heard, &said)
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
	a.say(line(t, "anyone?"))
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
