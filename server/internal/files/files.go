// Package files is what a world keeps of heavy files (DECISIONS 89): a
// model, an image, a video, each named by the hash of its content and never
// changed.
// A session that may build sends one; the world reads its kind off its bytes,
// names it and keeps it. Alone, the files sit in the world folder and the
// server serves them; mundos's bucket takes them when it hands a world one.
package files

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"sort"
)

// Cap is the most bytes one file may have: a phone photo is 3 to 8 MB, and
// a model this large is a build, not a model (docs/sketches/entities.md).
const Cap = 20 << 20

// ErrKind is what bytes of no kind the world takes hear.
var ErrKind = errors.New("files: no kind the world takes")

// named is a name a file may have: its hash and its kind, nothing a person
// chose.
var named = regexp.MustCompile(`^[0-9a-f]{64}\.[a-z0-9]{2,5}$`)

// Kind is the extension bytes earn, read off their first bytes and never off
// a name. False for bytes of no kind the world takes.
func Kind(head []byte) (string, bool) {
	switch {
	case bytes.HasPrefix(head, []byte("VOX ")):
		return "vox", true
	case bytes.HasPrefix(head, []byte("\x89PNG\r\n\x1a\n")):
		return "png", true
	case bytes.HasPrefix(head, []byte{0xff, 0xd8, 0xff}):
		return "jpg", true
	case len(head) >= 12 && bytes.Equal(head[:4], []byte("RIFF")) && bytes.Equal(head[8:12], []byte("WEBP")):
		return "webp", true
	case bytes.HasPrefix(head, []byte("GIF87a")), bytes.HasPrefix(head, []byte("GIF89a")):
		return "gif", true
	// An MP4 and an M4A open with a box whose type is `ftyp`, after its four
	// byte size; the brand after says which.
	case len(head) >= 12 && bytes.Equal(head[4:8], []byte("ftyp")) && bytes.Equal(head[8:12], []byte("M4A ")):
		return "m4a", true
	case len(head) >= 8 && bytes.Equal(head[4:8], []byte("ftyp")):
		return "mp4", true
	// An MP3 opens with an ID3 tag, or straight with a frame's sync bits.
	case bytes.HasPrefix(head, []byte("ID3")), len(head) >= 2 && head[0] == 0xff && head[1]&0xe0 == 0xe0:
		return "mp3", true
	case bytes.HasPrefix(head, []byte("OggS")):
		return "ogg", true
	}
	return "", false
}

// Kept is one file the world holds.
type Kept struct {
	// `<hash>.<ext>`: what a record names after `asset://`.
	Name  string
	Bytes int64
}

// Files is the folder the files are kept in.
type Files struct {
	dir string
}

// Open makes the folder when it is missing.
func Open(dir string) (*Files, error) {
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return nil, fmt.Errorf("files: %w", err)
	}
	return &Files{dir: dir}, nil
}

// Keep names bytes and keeps them, and says whether the world held them
// already. The file is written whole beside its name and moved into place,
// so a reader sees a file or none.
func (f *Files) Keep(content []byte) (Kept, bool, error) {
	ext, ok := Kind(content)
	if !ok {
		return Kept{}, false, ErrKind
	}
	sum := sha256.Sum256(content)
	kept := Kept{Name: hex.EncodeToString(sum[:]) + "." + ext, Bytes: int64(len(content))}
	full := filepath.Join(f.dir, kept.Name)
	if _, err := os.Stat(full); err == nil {
		return kept, true, nil
	}
	temp, err := os.CreateTemp(f.dir, ".keep-*")
	if err != nil {
		return Kept{}, false, fmt.Errorf("files: %w", err)
	}
	if _, err := temp.Write(content); err != nil {
		temp.Close()
		os.Remove(temp.Name())
		return Kept{}, false, fmt.Errorf("files: %w", err)
	}
	if err := temp.Sync(); err != nil {
		temp.Close()
		os.Remove(temp.Name())
		return Kept{}, false, fmt.Errorf("files: %w", err)
	}
	if err := temp.Close(); err != nil {
		os.Remove(temp.Name())
		return Kept{}, false, fmt.Errorf("files: %w", err)
	}
	if err := os.Rename(temp.Name(), full); err != nil {
		os.Remove(temp.Name())
		return Kept{}, false, fmt.Errorf("files: %w", err)
	}
	return kept, false, nil
}

// Path is where a named file is, for one the world holds. False for a name
// that is no file's, or one the world does not hold: nothing a request says
// reaches the folder but a hash and a kind.
func (f *Files) Path(name string) (string, bool) {
	if !named.MatchString(name) {
		return "", false
	}
	full := filepath.Join(f.dir, name)
	info, err := os.Stat(full)
	if err != nil || !info.Mode().IsRegular() {
		return "", false
	}
	return full, true
}

// List is every file the world holds, by name.
func (f *Files) List() ([]Kept, error) {
	entries, err := os.ReadDir(f.dir)
	if err != nil {
		return nil, fmt.Errorf("files: %w", err)
	}
	var kept []Kept
	for _, entry := range entries {
		if !named.MatchString(entry.Name()) {
			continue
		}
		info, err := entry.Info()
		if errors.Is(err, fs.ErrNotExist) {
			continue
		}
		if err != nil {
			return nil, fmt.Errorf("files: %w", err)
		}
		kept = append(kept, Kept{Name: entry.Name(), Bytes: info.Size()})
	}
	sort.Slice(kept, func(i, j int) bool { return kept[i].Name < kept[j].Name })
	return kept, nil
}
