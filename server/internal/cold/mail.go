package cold

import (
	_ "embed"
	"fmt"
	"html/template"
	"log"
	"net/url"
	"strings"

	"github.com/pocketbase/pocketbase/core"
)

// mailCopy is what the sign in email says, in one language. {APP_NAME} is
// filled at send time.
type mailCopy struct {
	Lang, Subject, Greeting, Intro, Button, CodeIntro, Expiry string
}

// The email speaks the reader's language: the web app sends it with the code
// request and the account remembers it (`users.locale`); an account that
// never said one reads English. A language enters here and in the web app's
// catalogue together.
var otpCopy = map[string]mailCopy{
	"en": {
		Lang:      "en",
		Subject:   "Sign in to {APP_NAME}",
		Greeting:  "Hi,",
		Intro:     "Open the link to sign in to {APP_NAME}:",
		Button:    "Sign in",
		CodeIntro: "Signing in on another device? Type this code there:",
		Expiry:    "The link and the code work once and expire in a few minutes. If you did not ask for them, ignore this email.",
	},
	"pt": {
		Lang:      "pt",
		Subject:   "Entrar no {APP_NAME}",
		Greeting:  "Olá,",
		Intro:     "Abra o link para entrar no {APP_NAME}:",
		Button:    "Entrar",
		CodeIntro: "Entrando em outro aparelho? Digite este código lá:",
		Expiry:    "O link e o código valem uma vez e expiram em poucos minutos. Se você não pediu, ignore este e-mail.",
	},
}

const defaultMailLocale = "en"

// The layout is the web app's design system in its light mode: black on
// white, one hairline, no radius, JetBrains Mono where the reader has it and
// the system's mono where not, since no webfont reaches an inbox reliably.
// Light only, for the mail clients that ignore a dark scheme; tables, for
// the ones that ignore CSS layout. The code is the one thing set large: it
// is read on this screen and typed on another.
//
//go:embed otp_mail.html
var otpMailHTML string

var otpMail = template.Must(template.New("otp").Parse(otpMailHTML))

type otpMailData struct {
	AppName, Link, Code string
	Words               mailCopy
}

// knownLocale says whether the email exists in a language.
func knownLocale(locale string) bool {
	_, ok := otpCopy[locale]
	return ok
}

// mailLocale is the language an account reads, as far as it has said.
func mailLocale(record *core.Record) string {
	if locale := record.GetString("locale"); knownLocale(locale) {
		return locale
	}
	return defaultMailLocale
}

// say fills the app's name into one language's copy.
func say(words mailCopy, appName string) mailCopy {
	fill := func(s string) string { return strings.ReplaceAll(s, "{APP_NAME}", appName) }
	words.Subject, words.Intro = fill(words.Subject), fill(words.Intro)
	return words
}

// spaced splits a code in two halves, "3941 8276": read in two breaths and
// typed as one, since the code page drops the space.
func spaced(code string) string {
	if n := len(code); n >= 6 && n%2 == 0 {
		return code[:n/2] + " " + code[n/2:]
	}
	return code
}

// magicLink is the one click sign in URL. The web app owns the route.
func magicLink(appURL, otpID, code string) string {
	return fmt.Sprintf("%s/login/verify?otpId=%s&code=%s", appURL, url.QueryEscape(otpID), url.QueryEscape(code))
}

// writeMagicLinkMail renders the sign in email in the reader's language and
// puts it in the message in place of the collection's own template: one
// layout, the link as a button, the code in digits large enough to read from
// another screen. The collection template is what PocketBase would send with
// this hook unbound.
func writeMagicLinkMail(e *core.MailerRecordEvent) error {
	otpID, _ := e.Meta["otpId"].(string)
	code, _ := e.Meta["password"].(string)
	settings := e.App.Settings()
	words := say(otpCopy[mailLocale(e.Record)], settings.Meta.AppName)

	var body strings.Builder
	err := otpMail.Execute(&body, otpMailData{
		AppName: settings.Meta.AppName,
		Link:    magicLink(settings.Meta.AppURL, otpID, code),
		Code:    spaced(code),
		Words:   words,
	})
	if err != nil {
		return fmt.Errorf("render the sign in email: %w", err)
	}
	e.Message.Subject = words.Subject
	e.Message.HTML = body.String()
	return e.Next()
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
