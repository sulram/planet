// Package store is what an owner keeps of a world (DECISIONS 89, 110): the
// core's systems and each plugin, one SQLite file apiece in the world folder,
// rows of a key and a value the server never reads. The owner is the module's
// side of the bridge; this package carries bytes.
package store

import (
	"bytes"
	"database/sql"
	"errors"
	"fmt"
	"path/filepath"
	"regexp"
	"sync"

	_ "modernc.org/sqlite"
)

// An owner's name is a file's name: a to z and `_`, as a plugin's is.
var owned = regexp.MustCompile(`^[a-z_]+$`)

// Row is one thing kept.
type Row struct {
	Key   []byte
	Value []byte
}

// Write keeps a value under a key, or forgets the key.
type Write struct {
	Key    []byte
	Value  []byte
	Forget bool
}

// Store holds the files of one world folder, each opened when its owner
// first asks.
type Store struct {
	dir   string
	mu    sync.Mutex
	files map[string]*sql.DB
}

// Open names the folder the files are kept in. Nothing is opened until an
// owner asks.
func Open(dir string) *Store {
	return &Store{dir: dir, files: map[string]*sql.DB{}}
}

// file opens an owner's file, making it the first time.
func (s *Store) file(owner string) (*sql.DB, error) {
	if !owned.MatchString(owner) {
		return nil, fmt.Errorf("store: %q is no owner's name", owner)
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	if db, ok := s.files[owner]; ok {
		return db, nil
	}
	// One file and no log beside it, so the folder copies whole (DEPLOY).
	db, err := sql.Open("sqlite", filepath.Join(s.dir, owner+".sqlite")+"?_pragma=busy_timeout(5000)")
	if err != nil {
		return nil, fmt.Errorf("store: %w", err)
	}
	db.SetMaxOpenConns(1)
	if _, err := db.Exec(`CREATE TABLE IF NOT EXISTS kept (key BLOB PRIMARY KEY, value BLOB NOT NULL) WITHOUT ROWID`); err != nil {
		db.Close()
		return nil, fmt.Errorf("store: %s: %w", owner, err)
	}
	s.files[owner] = db
	return db, nil
}

// Get is what an owner keeps under a key. False when it keeps nothing there.
func (s *Store) Get(owner string, key []byte) ([]byte, bool, error) {
	db, err := s.file(owner)
	if err != nil {
		return nil, false, err
	}
	var value []byte
	switch err := db.QueryRow(`SELECT value FROM kept WHERE key = ?`, key).Scan(&value); {
	case errors.Is(err, sql.ErrNoRows):
		return nil, false, nil
	case err != nil:
		return nil, false, fmt.Errorf("store: %s: %w", owner, err)
	}
	return value, true, nil
}

// Scan is every row of an owner whose key starts with a prefix, in the
// order of the keys.
func (s *Store) Scan(owner string, prefix []byte) ([]Row, error) {
	db, err := s.file(owner)
	if err != nil {
		return nil, err
	}
	rows, err := db.Query(`SELECT key, value FROM kept WHERE key >= ? ORDER BY key`, prefix)
	if err != nil {
		return nil, fmt.Errorf("store: %s: %w", owner, err)
	}
	defer rows.Close()
	var found []Row
	for rows.Next() {
		var row Row
		if err := rows.Scan(&row.Key, &row.Value); err != nil {
			return nil, fmt.Errorf("store: %s: %w", owner, err)
		}
		// The keys come in order: past the prefix, none starts with it.
		if !bytes.HasPrefix(row.Key, prefix) {
			break
		}
		found = append(found, row)
	}
	return found, rows.Err()
}

// Apply writes what one call kept, all of it or none.
func (s *Store) Apply(owner string, writes []Write) error {
	if len(writes) == 0 {
		return nil
	}
	db, err := s.file(owner)
	if err != nil {
		return err
	}
	tx, err := db.Begin()
	if err != nil {
		return fmt.Errorf("store: %s: %w", owner, err)
	}
	for _, write := range writes {
		if write.Forget {
			_, err = tx.Exec(`DELETE FROM kept WHERE key = ?`, write.Key)
		} else {
			// An empty value is kept as one: NOT NULL holds no nil.
			_, err = tx.Exec(`INSERT INTO kept (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value`, write.Key, append([]byte{}, write.Value...))
		}
		if err != nil {
			_ = tx.Rollback()
			return fmt.Errorf("store: %s: %w", owner, err)
		}
	}
	if err := tx.Commit(); err != nil {
		return fmt.Errorf("store: %s: %w", owner, err)
	}
	return nil
}

// Close shuts every file.
func (s *Store) Close() error {
	s.mu.Lock()
	defer s.mu.Unlock()
	var first error
	for owner, db := range s.files {
		if err := db.Close(); err != nil && first == nil {
			first = err
		}
		delete(s.files, owner)
	}
	return first
}
