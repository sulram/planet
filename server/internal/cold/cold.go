// Package cold is the cold plane: accounts, worlds and the operator flag, kept
// in PocketBase. All PocketBase glue lives here so the world core never sees a
// PocketBase type.
package cold

import (
	"github.com/pocketbase/pocketbase/core"

	"github.com/sulram/planet/server/internal/world"
	// Registers the schema migrations; PocketBase applies them on serve.
	_ "github.com/sulram/planet/server/migrations"
)

const (
	usersCollection    = "users"
	worldsCollection   = "worlds"
	instanceCollection = "instance"
)

// Register binds the cold plane to the app, and the hot plane's doors with
// it. Nothing touches the database until the app serves, after the
// migrations have run.
func Register(app core.App, cfg Config) {
	hub := world.NewHub(catalog{app: app})
	tickets := world.NewTickets()
	app.OnServe().BindFunc(func(e *core.ServeEvent) error {
		if err := applySettings(e.App, cfg); err != nil {
			return err
		}
		registerHot(e, hub, tickets)
		if err := ensureOperator(e.App, cfg.OperatorEmail); err != nil {
			return err
		}
		if err := ensureInstance(e.App); err != nil {
			return err
		}
		return e.Next()
	})

	app.OnRecordRequestOTPRequest(usersCollection).BindFunc(createAccountOnFirstLogin)
	app.OnRecordRequestOTPRequest(usersCollection).BindFunc(rememberLocale)
	app.OnRecordEnrich(usersCollection).BindFunc(showEmailsToOperators)
	app.OnRecordValidate(worldsCollection).BindFunc(validateWorld)
	app.OnRecordCreate(instanceCollection).BindFunc(keepOneInstance)
	app.OnMailerRecordOTPSend(usersCollection).BindFunc(writeMagicLinkMail)
	app.OnMailerRecordOTPSend(usersCollection).BindFunc(logMagicLink)
	app.OnMailerSend().BindFunc(skipDeliveryWithoutSMTP)
}
