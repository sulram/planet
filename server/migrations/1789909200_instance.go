package migrations

import (
	"github.com/pocketbase/pocketbase/core"
	m "github.com/pocketbase/pocketbase/migrations"
	"github.com/pocketbase/pocketbase/tools/types"
)

// The instance: one record, what is the server's and not a world's. Its
// first field is the front door. The cold plane seeds the
// record on start and a hook refuses a second, so the row is read, never
// created, by anyone else. Worlds are made by operators from here on.
func init() {
	m.Register(func(app core.App) error {
		worlds, err := app.FindCollectionByNameOrId("worlds")
		if err != nil {
			return err
		}

		instance := core.NewBaseCollection("instance")
		instance.Fields.Add(
			// Not required and not cascading: deleting the main world clears
			// the pointer, and the instance is then not open yet.
			&core.RelationField{Name: "main_world", CollectionId: worlds.Id, MaxSelect: 1},
			&core.AutodateField{Name: "updated", OnCreate: true, OnUpdate: true},
		)
		// Everyone reads the front door; only an operator moves it. Nobody
		// creates or deletes the row through the API.
		instance.ListRule = types.Pointer("")
		instance.ViewRule = types.Pointer("")
		instance.CreateRule = nil
		instance.UpdateRule = types.Pointer("@request.auth.operator = true")
		instance.DeleteRule = nil
		if err := app.Save(instance); err != nil {
			return err
		}

		worlds.CreateRule = types.Pointer("@request.auth.operator = true && owner = @request.auth.id")
		return app.Save(worlds)
	}, func(app core.App) error {
		worlds, err := app.FindCollectionByNameOrId("worlds")
		if err != nil {
			return err
		}
		worlds.CreateRule = types.Pointer("@request.auth.id != '' && owner = @request.auth.id")
		if err := app.Save(worlds); err != nil {
			return err
		}
		instance, err := app.FindCollectionByNameOrId("instance")
		if err != nil {
			return err
		}
		return app.Delete(instance)
	})
}
