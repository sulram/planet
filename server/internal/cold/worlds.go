package cold

import (
	"errors"

	validation "github.com/pocketbase/ozzo-validation/v4"
	"github.com/pocketbase/pocketbase/core"

	"github.com/sulram/planet/server/internal/world"
)

// validateWorld lets the world core judge the recipe and freezes it after
// creation. It runs on every validated save, so the rule also holds for
// superusers and for Go code, which API rules never see.
func validateWorld(e *core.RecordEvent) error {
	// Field level checks first: they give the more specific messages.
	if err := e.Next(); err != nil {
		return err
	}

	recipe, err := recipeOf(e.Record)
	if err != nil {
		return err
	}
	if e.Record.IsNew() {
		return nil
	}

	original, err := recipeOf(e.Record.Original())
	if err != nil {
		return err
	}
	if !recipe.Equal(original) {
		frozen := validation.NewError("validation_recipe_frozen", "The recipe of a world cannot change after creation.")
		return validation.Errors{"seed": frozen, "generator_version": frozen, "params": frozen}
	}
	return nil
}

// recipeOf reads the recipe fields of a worlds record and reports a failure
// on the field that caused it.
func recipeOf(record *core.Record) (world.Recipe, error) {
	recipe, err := world.NewRecipe(
		record.GetString("seed"),
		record.GetInt("generator_version"),
		[]byte(record.GetString("params")),
	)

	field := ""
	switch {
	case err == nil:
		return recipe, nil
	case errors.Is(err, world.ErrSeed):
		field = "seed"
	case errors.Is(err, world.ErrGeneratorVersion):
		field = "generator_version"
	case errors.Is(err, world.ErrParams):
		field = "params"
	default:
		return world.Recipe{}, err
	}
	return world.Recipe{}, validation.Errors{field: validation.NewError("validation_invalid_recipe", err.Error())}
}
