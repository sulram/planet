package api

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/sulram/planet/server/internal/files"
)

// post sends bytes as a file, with a key.
func post(t *testing.T, web *httptest.Server, key string, content []byte) (int, map[string]any) {
	t.Helper()
	request, _ := http.NewRequest("POST", web.URL+"/api/files", bytes.NewReader(content))
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

func TestABuilderUploadsAFileAndTheWorldServesItByItsHash(t *testing.T) {
	web, private := hosted(t)
	builder := enter(t, web, token(t, private, "builder"))["key"].(string)
	visitor := enter(t, web, token(t, private, "signed_in"))["key"].(string)
	model := []byte("VOX \x96\x00\x00\x00MAIN\x00\x00\x00\x00\x00\x00\x00\x00")

	if status, why := post(t, web, "", model); status != http.StatusForbidden || why["error"] != "level" {
		t.Fatalf("a guest sends no file: %d %v", status, why)
	}
	if status, why := post(t, web, visitor, model); status != http.StatusForbidden || why["error"] != "level" {
		t.Fatalf("a signed in person with no role sends no file: %d %v", status, why)
	}
	if status, why := post(t, web, "a-key-nobody-holds", model); status != http.StatusUnauthorized || why["error"] != "key" {
		t.Fatalf("an unknown key: %d %v", status, why)
	}
	if status, why := post(t, web, builder, []byte("just text")); status != http.StatusUnsupportedMediaType || why["error"] != "kind" {
		t.Fatalf("bytes of no kind: %d %v", status, why)
	}
	if status, why := post(t, web, builder, append([]byte("VOX "), make([]byte, files.Cap)...)); status != http.StatusRequestEntityTooLarge || why["error"] != "size" {
		t.Fatalf("over the cap: %d %v", status, why)
	}

	status, kept := post(t, web, builder, model)
	name, _ := kept["asset"].(string)
	if status != http.StatusCreated || !strings.HasPrefix(name, "asset://") || !strings.HasSuffix(name, ".vox") || kept["bytes"] != float64(len(model)) {
		t.Fatalf("a builder's file is kept and named: %d %v", status, kept)
	}
	if status, again := post(t, web, builder, model); status != http.StatusOK || again["asset"] != name {
		t.Fatalf("the same bytes are the same file: %d %v", status, again)
	}

	_, about := call(t, "GET", web.URL+"/api/world", "", nil)
	if about["files"] != "/files" {
		t.Fatalf("the world says where its files are: %v", about)
	}
	response, err := http.Get(web.URL + "/files/" + strings.TrimPrefix(name, "asset://"))
	if err != nil {
		t.Fatal(err)
	}
	body, _ := io.ReadAll(response.Body)
	response.Body.Close()
	if response.StatusCode != http.StatusOK || !bytes.Equal(body, model) {
		t.Fatalf("the file is served: %d %q", response.StatusCode, body)
	}
	if response.Header.Get("Cache-Control") != forGood {
		t.Fatalf("for good: %s", response.Header.Get("Cache-Control"))
	}
	for _, path := range []string{"/files/world.json", "/files/..%2Fworld.json", "/files/" + strings.Repeat("0", 64) + ".vox"} {
		response, err := http.Get(web.URL + path)
		if err != nil {
			t.Fatal(err)
		}
		response.Body.Close()
		if response.StatusCode != http.StatusNotFound {
			t.Fatalf("%s is nothing: %d", path, response.StatusCode)
		}
	}

	if status, _ := call(t, "GET", web.URL+"/api/files", visitor, nil); status != http.StatusForbidden {
		t.Fatalf("the list is a builder's: %d", status)
	}
	request, _ := http.NewRequest("GET", web.URL+"/api/files", nil)
	request.Header.Set("Authorization", "Bearer "+builder)
	response, err = http.DefaultClient.Do(request)
	if err != nil {
		t.Fatal(err)
	}
	var held []map[string]any
	_ = json.NewDecoder(response.Body).Decode(&held)
	response.Body.Close()
	if len(held) != 1 || held[0]["asset"] != name {
		t.Fatalf("the world holds one file: %v", held)
	}
}
