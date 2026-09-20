package cold

import "github.com/pocketbase/pocketbase/core"

// applySettings writes the environment into the PocketBase settings. It sets
// plain values, so running it on every start is idempotent.
func applySettings(app core.App, cfg Config) error {
	settings := app.Settings()

	settings.Meta.AppName = cfg.AppName
	settings.Meta.AppURL = cfg.AppURL
	settings.Meta.SenderName = cfg.AppName
	settings.Meta.SenderAddress = cfg.SenderAddress

	settings.SMTP.Enabled = cfg.SMTP.Host != ""
	settings.SMTP.Host = cfg.SMTP.Host
	settings.SMTP.Port = cfg.SMTP.Port
	settings.SMTP.Username = cfg.SMTP.Username
	settings.SMTP.Password = cfg.SMTP.Password

	return app.Save(settings)
}
