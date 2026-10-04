// Package folder is the world folder: where a world keeps itself between
// runs. It holds the recipe the founding froze; each plugin that keeps things
// has a file of its own beside it.
package folder

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/sulram/planet/server/internal/world"
)

// ErrFounded is what a second founding hears: a recipe is written once.
var ErrFounded = errors.New("the world is already founded")

// fileName is the one file of the core. Its absence is an unfounded world.
const fileName = "world.json"

// Folder is one world's folder. It is the world core's Catalog.
type Folder struct {
	dir string

	mu sync.Mutex
	// Nil while the world is unfounded. Frozen once set, so it is read once.
	recipe *world.Recipe
}

// file is world.json. The recipe has the wire's shape, so it travels unmapped.
type file struct {
	Recipe  recipe    `json:"recipe"`
	Founded time.Time `json:"founded"`
}

type recipe struct {
	Seed             string          `json:"seed"`
	GeneratorVersion int             `json:"generator_version"`
	Params           json.RawMessage `json:"params"`
}

// Open makes the folder when it is missing and reads the recipe when the
// world is founded. A file that does not read is an error, never an
// unfounded world: founding over it would lose a world.
func Open(dir string) (*Folder, error) {
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return nil, fmt.Errorf("world folder: %w", err)
	}
	f := &Folder{dir: dir}
	raw, err := os.ReadFile(filepath.Join(dir, fileName))
	if errors.Is(err, fs.ErrNotExist) {
		return f, nil
	}
	if err != nil {
		return nil, fmt.Errorf("world folder: %w", err)
	}
	var kept file
	if err := json.Unmarshal(raw, &kept); err != nil {
		return nil, fmt.Errorf("world folder: %s: %w", fileName, err)
	}
	read, err := world.NewRecipe(kept.Recipe.Seed, kept.Recipe.GeneratorVersion, kept.Recipe.Params)
	if err != nil {
		return nil, fmt.Errorf("world folder: %s: %w", fileName, err)
	}
	f.recipe = &read
	return f, nil
}

// Recipe is the world's recipe, or world.ErrUnfounded while it has none.
func (f *Folder) Recipe(context.Context) (world.Recipe, error) {
	f.mu.Lock()
	defer f.mu.Unlock()
	if f.recipe == nil {
		return world.Recipe{}, world.ErrUnfounded
	}
	return *f.recipe, nil
}

// Found writes the recipe, once. The file is written whole beside its name
// and linked into place, so a crash leaves a founded world or an unfounded
// one, never half of one, and a link over an existing file fails: two
// foundings at once leave one world.
func (f *Folder) Found(founded world.Recipe) error {
	f.mu.Lock()
	defer f.mu.Unlock()
	if f.recipe != nil {
		return ErrFounded
	}

	raw, err := json.MarshalIndent(file{
		Recipe: recipe{
			Seed:             founded.Seed.String(),
			GeneratorVersion: founded.GeneratorVersion,
			Params:           founded.Params,
		},
		Founded: time.Now().UTC().Truncate(time.Second),
	}, "", "\t")
	if err != nil {
		return err
	}

	draft, err := os.CreateTemp(f.dir, fileName+".*")
	if err != nil {
		return fmt.Errorf("found: %w", err)
	}
	defer os.Remove(draft.Name())
	if _, err := draft.Write(append(raw, '\n')); err != nil {
		draft.Close()
		return fmt.Errorf("found: %w", err)
	}
	if err := draft.Sync(); err != nil {
		draft.Close()
		return fmt.Errorf("found: %w", err)
	}
	if err := draft.Close(); err != nil {
		return fmt.Errorf("found: %w", err)
	}
	if err := os.Chmod(draft.Name(), 0o644); err != nil {
		return fmt.Errorf("found: %w", err)
	}
	if err := os.Link(draft.Name(), filepath.Join(f.dir, fileName)); err != nil {
		if errors.Is(err, fs.ErrExist) {
			return ErrFounded
		}
		return fmt.Errorf("found: %w", err)
	}
	f.recipe = &founded
	return nil
}
