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
