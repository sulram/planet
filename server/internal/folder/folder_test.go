package folder

import (
	"context"
	"errors"
	"os"
	"path/filepath"
	"testing"

	"github.com/sulram/planet/server/internal/world"
)

func aRecipe(t *testing.T, seed string) world.Recipe {
	t.Helper()
	r, err := world.NewRecipe(seed, 3, []byte(`{"relief_m":2000}`))
	if err != nil {
		t.Fatal(err)
	}
	return r
}

func TestAWorldIsFoundedOnce(t *testing.T) {
	dir := filepath.Join(t.TempDir(), "world")
	f, err := Open(dir)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := f.Recipe(context.Background()); !errors.Is(err, world.ErrUnfounded) {
		t.Fatalf("a new folder is an unfounded world: %v", err)
	}

	first := aRecipe(t, "00000000deadbeef")
	if err := f.Found(first); err != nil {
		t.Fatal(err)
	}
	if err := f.Found(aRecipe(t, "0000000000000001")); !errors.Is(err, ErrFounded) {
		t.Fatalf("a second founding is refused: %v", err)
	}

	// The folder is the world: another run reads the same recipe.
	again, err := Open(dir)
	if err != nil {
		t.Fatal(err)
	}
	kept, err := again.Recipe(context.Background())
	if err != nil || !kept.Equal(first) {
		t.Fatalf("the recipe outlives the process: %v %v", kept, err)
	}
	if err := again.Found(aRecipe(t, "0000000000000001")); !errors.Is(err, ErrFounded) {
		t.Fatalf("and stays frozen: %v", err)
	}
}

func TestTwoFoldersOverOneWorldFoundItOnce(t *testing.T) {
	dir := t.TempDir()
	a, _ := Open(dir)
	b, _ := Open(dir)
	if err := a.Found(aRecipe(t, "00000000deadbeef")); err != nil {
		t.Fatal(err)
	}
	if err := b.Found(aRecipe(t, "0000000000000001")); !errors.Is(err, ErrFounded) {
		t.Fatalf("the file on disk refuses the second: %v", err)
	}
	if left, _ := filepath.Glob(filepath.Join(dir, "world.json.*")); len(left) != 0 {
		t.Fatalf("no draft is left behind: %v", left)
	}
}

func TestAFileThatDoesNotReadIsNoUnfoundedWorld(t *testing.T) {
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "world.json"), []byte(`{"recipe":{"seed":"nope"}}`), 0o644); err != nil {
		t.Fatal(err)
	}
	if _, err := Open(dir); err == nil {
		t.Fatal("a broken file must stop the server, or a founding would write over a world")
	}
}

func TestAWorldIsCopiedIntoItsNextGeneration(t *testing.T) {
	from := t.TempDir()
	source, _ := Open(from)
	first := aRecipe(t, "00000000deadbeef")
	if err := source.Found(first); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(from, "plugin"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(from, "plugin", "kept"), []byte("a trace"), 0o644); err != nil {
		t.Fatal(err)
	}

	to := filepath.Join(t.TempDir(), "next")
	if err := Copy(from, to); err != nil {
		t.Fatal(err)
	}
	clone, err := Open(to)
	if err != nil {
		t.Fatal(err)
	}
	if kept, err := clone.Recipe(context.Background()); err != nil || !kept.Equal(first) {
		t.Fatalf("the copy is the same world: %v %v", kept, err)
	}
	if trace, _ := os.ReadFile(filepath.Join(to, "plugin", "kept")); string(trace) != "a trace" {
		t.Fatalf("with everything its folder held: %q", trace)
	}

	if err := Copy(from, to); err == nil {
		t.Fatal("a copy over a world is refused")
	}
	if err := Copy(filepath.Join(from, "nowhere"), filepath.Join(t.TempDir(), "x")); err == nil {
		t.Fatal("a copy of nothing is refused")
	}

	// An unfounded world copies to an unfounded world.
	empty := filepath.Join(t.TempDir(), "empty")
	if err := Copy(t.TempDir(), empty); err != nil {
		t.Fatal(err)
	}
	blank, _ := Open(empty)
	if _, err := blank.Recipe(context.Background()); !errors.Is(err, world.ErrUnfounded) {
		t.Fatalf("an empty folder copies to an unfounded world: %v", err)
	}
}
