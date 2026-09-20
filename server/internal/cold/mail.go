package cold

import (
	"fmt"
	"log"
	"net/url"

	"github.com/pocketbase/pocketbase/core"
)

// magicLink is the one click sign in URL, the same one the OTP email template
// spells with placeholders. The web app owns the route.
func magicLink(appURL, otpID, code string) string {
	return fmt.Sprintf("%s/login/verify?otpId=%s&code=%s", appURL, url.QueryEscape(otpID), url.QueryEscape(code))
}

// logMagicLink prints the link and the code when mail is off, so development
// needs no mail catcher. It writes to the console and not to the app logger:
// that one persists to the logs database, no place for a sign in secret.
func logMagicLink(e *core.MailerRecordEvent) error {
	if !e.App.Settings().SMTP.Enabled {
		otpID, _ := e.Meta["otpId"].(string)
		code, _ := e.Meta["password"].(string)
		log.Printf("SMTP is off, nothing was emailed. Sign in for %s:\n  link: %s\n  code: %s",
			e.Record.Email(), magicLink(e.App.Settings().Meta.AppURL, otpID, code), code)
	}
	return e.Next()
}

// skipDeliveryWithoutSMTP ends the send chain before the transport when mail
// is off. PocketBase would otherwise fall back to the local sendmail binary,
// which is absent on most machines and would fail every OTP request.
func skipDeliveryWithoutSMTP(e *core.MailerEvent) error {
	if e.App.Settings().SMTP.Enabled {
		return e.Next()
	}
	return nil
}
