package api

import (
	"bytes"
	"crypto/ed25519"
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/sulram/planet/server/internal/folder"
	"github.com/sulram/planet/server/internal/world"
)

var now = time.Unix(1790000000, 0)

// instance is a server over a new, unfounded world, as mundos would start it
// when `private` is kept, or alone when cfg has no key.
func instance(t *testing.T, cfg Config) *httptest.Server {
	t.Helper()
	kept, err := folder.Open(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	server := New(cfg, kept)
	server.now = func() time.Time { return now }
	web := httptest.NewServer(server.Handler())
	t.Cleanup(web.Close)
	return web
}

func hosted(t *testing.T) (*httptest.Server, ed25519.PrivateKey) {
	t.Helper()
	public, private, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	return instance(t, Config{Name: "oficina", MundosURL: "https://mundos.example", PublicKey: public, Version: "test"}), private
}

// token is what mundos would sign for this world.
func token(t *testing.T, private ed25519.PrivateKey, level string) string {
	t.Helper()
	part := func(v any) string {
		raw, _ := json.Marshal(v)
		return base64.RawURLEncoding.EncodeToString(raw)
	}
	signed := part(map[string]string{"alg": "EdDSA", "typ": "JWT"}) + "." + part(map[string]any{
		"iss": "mundos", "sub": "acc1", "aud": "oficina", "name": "Ada", "level": level, "exp": now.Unix() + 60,
	})
	return signed + "." + base64.RawURLEncoding.EncodeToString(ed25519.Sign(private, []byte(signed)))
}

func call(t *testing.T, method, url, key string, body any) (int, map[string]any) {
	t.Helper()
	raw, _ := json.Marshal(body)
	request, _ := http.NewRequest(method, url, bytes.NewReader(raw))
	if key != "" {
		request.Header.Set("Authorization", "Bearer "+key)
	}
	response, err := http.DefaultClient.Do(request)
	if err != nil {
		t.Fatal(err)
	}
	defer response.Body.Close()
	var answer map[string]any
	_ = json.NewDecoder(response.Body).Decode(&answer)
	return response.StatusCode, answer
}

func enter(t *testing.T, web *httptest.Server, identity string) map[string]any {
	t.Helper()
	status, who := call(t, "POST", web.URL+"/api/enter", "", map[string]string{"identity": identity})
	if status != http.StatusOK {
		t.Fatalf("enter answered %d", status)
	}
	return who
}

var earth = map[string]any{"seed": "00000000deadbeef", "generator_version": 3, "params": map[string]any{"relief_m": 2000}}

func TestAWorldIsBornUnfoundedAndItsAdminFoundsIt(t *testing.T) {
	web, private := hosted(t)

	_, about := call(t, "GET", web.URL+"/api/world", "", nil)
	if about["founded"] != false || about["recipe"] != nil || about["name"] != "oficina" {
		t.Fatalf("a new world has no recipe: %v", about)
	}
	if about["door"] != "https://mundos.example/enter" || about["protocol"] != float64(world.Protocol) {
		t.Fatalf("it says where the door is and what it speaks: %v", about)
	}

	guest := enter(t, web, "guest")
	if guest["level"] != "anonymous" || guest["key"] != "" {
		t.Fatalf("a guest is anonymous and holds no key: %v", guest)
	}
	if forged := enter(t, web, "not.a.token"); forged["level"] != "anonymous" || forged["key"] != "" {
		t.Fatalf("a token that does not check is a visitor: %v", forged)
	}
	builder := enter(t, web, token(t, private, "builder"))
	if builder["level"] != "builder" || builder["name"] != "Ada" || builder["key"] == "" {
		t.Fatalf("a signed person is who mundos said: %v", builder)
	}
	admin := enter(t, web, token(t, private, "admin"))
	if status, me := call(t, "GET", web.URL+"/api/me", builder["key"].(string), nil); status != http.StatusOK || me["level"] != "builder" {
		t.Fatalf("a key that stands says whose it is: %d %v", status, me)
	}
	if status, _ := call(t, "GET", web.URL+"/api/me", "a-key-nobody-holds", nil); status != http.StatusUnauthorized {
		t.Fatalf("a key nobody holds stands for nobody: %d", status)
	}

	for who, key := range map[string]string{"a visitor": "", "a builder": builder["key"].(string)} {
		if status, _ := call(t, "POST", web.URL+"/api/world", key, earth); status != http.StatusForbidden {
			t.Fatalf("%s founding answered %d", who, status)
		}
	}
	if status, _ := call(t, "POST", web.URL+"/api/world", "a-key-nobody-holds", earth); status != http.StatusUnauthorized {
		t.Fatalf("an unknown key answered %d", status)
	}
	adminKey := admin["key"].(string)
	if status, why := call(t, "POST", web.URL+"/api/world", adminKey, map[string]any{"seed": "nope", "generator_version": 3}); status != http.StatusBadRequest || why["error"] != "seed" {
		t.Fatalf("a recipe that does not read says which part: %d %v", status, why)
	}

	status, made := call(t, "POST", web.URL+"/api/world", adminKey, earth)
	if status != http.StatusOK || made["founded"] != true {
		t.Fatalf("an admin founds the world: %d %v", status, made)
	}
	if recipe := made["recipe"].(map[string]any); recipe["seed"] != "00000000deadbeef" || recipe["generator_version"] != float64(3) {
		t.Fatalf("with the recipe it sent: %v", recipe)
	}
	other := map[string]any{"seed": "0000000000000001", "generator_version": 3}
	if status, why := call(t, "POST", web.URL+"/api/world", adminKey, other); status != http.StatusConflict || why["error"] != "founded" {
		t.Fatalf("a founded world is frozen: %d %v", status, why)
	}
	if _, about := call(t, "GET", web.URL+"/api/world", "", nil); about["recipe"].(map[string]any)["seed"] != "00000000deadbeef" {
		t.Fatalf("and keeps the first recipe: %v", about)
	}
}

func TestWithNoMundosEverySessionIsTheDevLevel(t *testing.T) {
	alone := instance(t, Config{DevLevel: world.LevelAdmin})
	if who := enter(t, alone, ""); who["level"] != "admin" || who["key"] != "" {
		t.Fatalf("no door, and the level of the variable: %v", who)
	}
	if _, about := call(t, "GET", alone.URL+"/api/world", "", nil); about["door"] != "" {
		t.Fatalf("a world alone has no door: %v", about)
	}
	if status, _ := call(t, "POST", alone.URL+"/api/world", "", earth); status != http.StatusOK {
		t.Fatalf("an admin by the variable founds the world: %d", status)
	}

	closed := instance(t, Config{})
	if status, _ := call(t, "POST", closed.URL+"/api/world", "", earth); status != http.StatusForbidden {
		t.Fatalf("with no variable nobody founds: %d", status)
	}
}

func TestTheSocketWantsAKeyThatStands(t *testing.T) {
	web, _ := hosted(t)
	response, err := http.Get(web.URL + "/api/socket?key=a-key-nobody-holds")
	if err != nil {
		t.Fatal(err)
	}
	response.Body.Close()
	if response.StatusCode != http.StatusUnauthorized {
		t.Fatalf("a key nobody holds opens no socket: %d", response.StatusCode)
	}
}

func TestTheFrontEndIsFiles(t *testing.T) {
	dir := t.TempDir()
	write := func(name, body string) {
		full := filepath.Join(dir, filepath.FromSlash(name))
		_ = os.MkdirAll(filepath.Dir(full), 0o755)
		if err := os.WriteFile(full, []byte(body), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	write("index.html", "the page")
	write("ds.html", "the catalogue")
	write("404.html", "nothing here")
	write("_app/immutable/chunk.js", "code")
	write("assets/fields/earth.field", "plain")
	write("assets/fields/earth.field.br", "packed")
	write("assets/avatars/Kyle.vrm", "vrm")
	web := instance(t, Config{WebDir: dir})

	get := func(path string, encoding string) (*http.Response, string) {
		request, _ := http.NewRequest("GET", web.URL+path, nil)
		request.Header.Set("Accept-Encoding", encoding)
		response, err := http.DefaultTransport.RoundTrip(request)
		if err != nil {
			t.Fatal(err)
		}
		defer response.Body.Close()
		var body bytes.Buffer
		_, _ = body.ReadFrom(response.Body)
		return response, body.String()
	}

	for path, want := range map[string]string{"/": "the page", "/ds": "the catalogue"} {
		if response, body := get(path, ""); response.StatusCode != 200 || body != want {
			t.Fatalf("%s is its file: %d %q", path, response.StatusCode, body)
		}
	}
	if response, body := get("/nowhere", ""); response.StatusCode != 404 || body != "nothing here" {
		t.Fatalf("an unknown path is the 404 page: %d %q", response.StatusCode, body)
	}
	if response, _ := get("/api/nowhere", ""); response.StatusCode != 404 || response.Header.Get("Content-Type") == "text/html; charset=utf-8" {
		t.Fatalf("an unknown route is no page: %d", response.StatusCode)
	}
	// The mux sends a path with dots to its clean form, which is a file of
	// the folder or nothing.
	if response, _ := get("/../../etc/passwd", ""); response.StatusCode/100 != 3 || response.Header.Get("Location") != "/etc/passwd" {
		t.Fatalf("no path climbs out of the folder: %d %q", response.StatusCode, response.Header.Get("Location"))
	}
	if response, _ := get("/etc/passwd", ""); response.StatusCode != 404 {
		t.Fatalf("and the clean path is no file of ours: %d", response.StatusCode)
	}

	if response, _ := get("/_app/immutable/chunk.js", ""); response.Header.Get("Cache-Control") != forGood {
		t.Fatalf("what Vite named by content is kept for good: %q", response.Header.Get("Cache-Control"))
	}
	if response, _ := get("/assets/avatars/Kyle.vrm", ""); response.Header.Get("Cache-Control") != "no-cache" {
		t.Fatalf("an avatar is asked about again: %q", response.Header.Get("Cache-Control"))
	}
	response, body := get("/assets/fields/earth.field?abc", "gzip, br")
	if body != "packed" || response.Header.Get("Content-Encoding") != "br" || response.Header.Get("Cache-Control") != forGood {
		t.Fatalf("a field travels as its brotli sibling, kept for good: %q %v", body, response.Header)
	}
	if response, body := get("/assets/fields/earth.field?abc", "gzip"); body != "plain" || response.Header.Get("Content-Encoding") != "" {
		t.Fatalf("and plain to who does not read brotli: %q %v", body, response.Header)
	}
}
