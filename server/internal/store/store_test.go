package store

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"
)

func TestAnOwnerKeepsReadsScansAndForgets(t *testing.T) {
	dir := t.TempDir()
	s := Open(dir)
	if _, found, err := s.Get("cells", []byte("v1")); err != nil || found {
		t.Fatalf("an owner that kept nothing finds nothing: %v %v", found, err)
	}
	writes := []Write{
		{Key: []byte("v1"), Value: []byte("one")},
		{Key: []byte("v2"), Value: nil},
		{Key: []byte{'v', 0xff}, Value: []byte("last")},
		{Key: []byte("w1"), Value: []byte("other")},
	}
	if err := s.Apply("cells", writes); err != nil {
		t.Fatal(err)
	}
	if value, found, _ := s.Get("cells", []byte("v1")); !found || string(value) != "one" {
		t.Fatalf("what was kept is read back: %q %v", value, found)
	}
	if value, found, _ := s.Get("cells", []byte("v2")); !found || len(value) != 0 {
		t.Fatalf("an empty value is kept as one: %q %v", value, found)
	}
	rows, err := s.Scan("cells", []byte("v"))
	if err != nil || len(rows) != 3 || !bytes.Equal(rows[2].Key, []byte{'v', 0xff}) {
		t.Fatalf("a prefix finds its rows in order, and no other: %v %v", rows, err)
	}
	// One owner's rows are no other's, and each has a file of its own.
	if rows, _ := s.Scan("chat", nil); len(rows) != 0 {
		t.Fatalf("another owner keeps nothing of it: %v", rows)
	}
	for _, owner := range []string{"cells", "chat"} {
		if _, err := os.Stat(filepath.Join(dir, owner+".sqlite")); err != nil {
			t.Fatalf("%s has its file in the folder: %v", owner, err)
		}
	}
	if err := s.Apply("cells", []Write{{Key: []byte("v1"), Value: []byte("uno")}, {Key: []byte("v2"), Forget: true}}); err != nil {
		t.Fatal(err)
	}
	if err := s.Close(); err != nil {
		t.Fatal(err)
	}

	// Another process over the same folder reads what the first kept.
	again := Open(dir)
	defer again.Close()
	if value, found, _ := again.Get("cells", []byte("v1")); !found || string(value) != "uno" {
		t.Fatalf("a value kept again replaces the first: %q %v", value, found)
	}
	if _, found, _ := again.Get("cells", []byte("v2")); found {
		t.Fatal("a key forgotten is gone")
	}
}

func TestAnOwnerIsNamedAsAPluginIs(t *testing.T) {
	s := Open(t.TempDir())
	defer s.Close()
	for _, owner := range []string{"", "../etc", "Cells", "a/b", "a.b"} {
		if _, _, err := s.Get(owner, []byte("k")); err == nil {
			t.Fatalf("%q is no owner's name", owner)
		}
	}
}
