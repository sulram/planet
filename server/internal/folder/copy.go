package folder

import (
	"errors"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
)

// Copy fills a new world folder from another: how a world reaches its next
// generation (docs/DEPLOY.md). The source may be a world that is running.
// Every file goes as it is: the recipe is written whole and linked into
// place, so a copy reads the old world or the new one, never half of one.
// A plugin's store, when the first one exists, goes through its own backup
// here, since a database is copied as a snapshot and not as bytes.
//
// The destination is new or empty. Copying over a world would leave a mix
// of two, so it is refused.
func Copy(from, to string) error {
	if _, err := os.Stat(from); err != nil {
		return fmt.Errorf("copy: %w", err)
	}
	if err := os.MkdirAll(to, 0o755); err != nil {
		return fmt.Errorf("copy: %w", err)
	}
	held, err := os.ReadDir(to)
	if err != nil {
		return fmt.Errorf("copy: %w", err)
	}
	if len(held) != 0 {
		return errors.New("copy: the destination already holds a world")
	}

	return filepath.WalkDir(from, func(path string, entry fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, err := filepath.Rel(from, path)
		if err != nil {
			return err
		}
		target := filepath.Join(to, rel)
		switch {
		case entry.IsDir():
			return os.MkdirAll(target, 0o755)
		case entry.Type().IsRegular():
			return copyFile(path, target)
		default:
			// A link or a device has no place in a world folder.
			return fmt.Errorf("copy: %s is no file", rel)
		}
	})
}

func copyFile(from, to string) error {
	source, err := os.Open(from)
	if err != nil {
		return err
	}
	defer source.Close()
	target, err := os.OpenFile(to, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o644)
	if err != nil {
		return err
	}
	if _, err := io.Copy(target, source); err != nil {
		target.Close()
		return err
	}
	if err := target.Sync(); err != nil {
		target.Close()
		return err
	}
	return target.Close()
}
