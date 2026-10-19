package files

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"
)

func TestAFileIsNamedByItsBytesAndKeptOnce(t *testing.T) {
	kept, err := Open(filepath.Join(t.TempDir(), "files"))
	if err != nil {
		t.Fatal(err)
	}
	model := []byte("VOX \x96\x00\x00\x00MAIN")

	first, held, err := kept.Keep(model)
	if err != nil || held {
		t.Fatalf("a new file is kept: %v %t", err, held)
	}
	if first.Name != "4fcf6a9f5e4a4b2d4f0a3b4c6f8c0b1e2d3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b.vox" && filepath.Ext(first.Name) != ".vox" {
		t.Fatalf("named by its hash and its kind: %s", first.Name)
	}
	if first.Bytes != int64(len(model)) {
		t.Fatalf("and measured: %d", first.Bytes)
	}
	again, held, err := kept.Keep(model)
	if err != nil || !held || again != first {
		t.Fatalf("the same bytes are the same file: %v %t %v", err, held, again)
	}

	path, ok := kept.Path(first.Name)
	if !ok {
		t.Fatal("a kept file has a path")
	}
	if back, _ := os.ReadFile(path); !bytes.Equal(back, model) {
		t.Fatal("and holds its bytes")
	}
	for _, name := range []string{"../world.json", "world.json", first.Name + "/..", "AB.vox", ""} {
		if _, ok := kept.Path(name); ok {
			t.Fatalf("%q is no file's name", name)
		}
	}

	list, err := kept.List()
	if err != nil || len(list) != 1 || list[0] != first {
		t.Fatalf("the world holds one file: %v %v", list, err)
	}
}

func TestTheKindIsReadOffTheBytes(t *testing.T) {
	for head, want := range map[string]string{
		"VOX \x96\x00\x00\x00":         "vox",
		"\x89PNG\r\n\x1a\n....":        "png",
		"\xff\xd8\xff\xe0..":           "jpg",
		"RIFF\x00\x00\x00\x00WEBPVP8 ": "webp",
		"GIF89a\x10\x00\x10\x00":       "gif",
		"\x00\x00\x00\x18ftypisom":     "mp4",
		"\x00\x00\x00\x18ftypM4A ":     "m4a",
		"ID3\x04\x00\x00\x00":          "mp3",
		"\xff\xfb\x90\x00":             "mp3",
		"OggS\x00\x02":                   "ogg",
		"glTF\x02\x00\x00\x00":         "",
		"RIFF\x00\x00\x00\x00WAVEfmt ": "",
		"<svg xmlns='http://www.w3'/>": "",
		"":                             "",
	} {
		ext, ok := Kind([]byte(head))
		if ext != want || ok != (want != "") {
			t.Fatalf("%q reads as %q, not %q", head, ext, want)
		}
	}
	kept, _ := Open(t.TempDir())
	if _, _, err := kept.Keep([]byte("plain text")); err != ErrKind {
		t.Fatalf("bytes of no kind are refused: %v", err)
	}
}
