package migrations

import (
	"github.com/pocketbase/pocketbase/core"
	m "github.com/pocketbase/pocketbase/migrations"
)

// The language an account reads, as the web app last sent it with a code
// request, so the sign in email speaks it. Written by a server hook and read
// by the mail hook, never through the API; empty reads as English.
func init() {
	m.Register(func(app core.App) error {
		users, err := app.FindCollectionByNameOrId("users")
		if err != nil {
			return err
		}
		users.Fields.Add(&core.TextField{Name: "locale", Max: 8, Hidden: true})
		return app.Save(users)
	}, func(app core.App) error {
		users, err := app.FindCollectionByNameOrId("users")
		if err != nil {
			return err
		}
		users.Fields.RemoveByName("locale")
		return app.Save(users)
	})
}
