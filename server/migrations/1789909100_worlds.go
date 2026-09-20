package migrations

import (
	"github.com/pocketbase/pocketbase/core"
	m "github.com/pocketbase/pocketbase/migrations"
	"github.com/pocketbase/pocketbase/tools/types"
)

func init() {
	m.Register(func(app core.App) error {
		users, err := app.FindCollectionByNameOrId("users")
		if err != nil {
			return err
		}

		worlds := core.NewBaseCollection("worlds")

		worlds.Fields.Add(
			&core.TextField{Name: "name", Required: true, Max: 80},
			// The recipe: seed, generator_version, params. A hook freezes it
			// after creation, for superusers too, which a rule cannot do.
			// The seed is a u64 as text: JSON and SQLite numbers stop at 2^53 and 2^63.
			&core.TextField{Name: "seed", Required: true, Pattern: "^[0-9a-f]{16}$"},
			&core.NumberField{Name: "generator_version", Required: true, OnlyInt: true, Min: types.Pointer(1.0)},
			&core.JSONField{Name: "params", MaxSize: 16 << 10},
			&core.RelationField{Name: "owner", Required: true, CollectionId: users.Id, MaxSelect: 1},
			&core.AutodateField{Name: "created", OnCreate: true},
			&core.AutodateField{Name: "updated", OnCreate: true, OnUpdate: true},
		)
		worlds.AddIndex("idx_worlds_owner", false, "`owner`", "")

		// Visitors may look, so reading is public.
		worlds.ListRule = types.Pointer("")
		worlds.ViewRule = types.Pointer("")
		worlds.CreateRule = types.Pointer("@request.auth.id != '' && owner = @request.auth.id")
		ownerOrOperator := "owner = @request.auth.id || @request.auth.operator = true"
		worlds.UpdateRule = types.Pointer(ownerOrOperator)
		worlds.DeleteRule = types.Pointer(ownerOrOperator)

		return app.Save(worlds)
	}, func(app core.App) error {
		worlds, err := app.FindCollectionByNameOrId("worlds")
		if err != nil {
			return err
		}
		return app.Delete(worlds)
	})
}
