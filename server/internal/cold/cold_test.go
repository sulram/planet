package cold

import (
	"html"
	"net/http"
	"strings"
	"testing"
	"time"

	"github.com/pocketbase/pocketbase/core"
	"github.com/pocketbase/pocketbase/tests"
	"github.com/pocketbase/pocketbase/tools/mailer"
)

var testConfig = Config{
	AppURL:        "http://planet.test",
	AppName:       "planet",
	SenderAddress: "planet@planet.test",
	// Mail is on so the test mailer captures it; nothing connects to this host.
	SMTP: SMTP{Host: "smtp.planet.test", Port: 587},
}

const validWorld = `"name":"Terra","seed":"0123456789abcdef","generator_version":1,"params":{"sea_level":0.4}`

// newApp returns a migrated app on an empty data dir with the cold plane
// registered, the way main builds it.
func newApp(t testing.TB) *tests.TestApp {
	t.Helper()
	app, err := tests.NewTestApp(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	Register(app, testConfig)
	return app
}

func createUser(t testing.TB, app core.App, email string, operator bool) *core.Record {
	t.Helper()
	users, err := app.FindCollectionByNameOrId(usersCollection)
	if err != nil {
		t.Fatal(err)
	}
	record := newUser(users, email)
	record.Set("operator", operator)
	if err := app.Save(record); err != nil {
		t.Fatal(err)
	}
	return record
}

func createWorld(t testing.TB, app core.App, owner *core.Record) *core.Record {
	t.Helper()
	worlds, err := app.FindCollectionByNameOrId(worldsCollection)
	if err != nil {
		t.Fatal(err)
	}
	record := core.NewRecord(worlds)
	record.Set("name", "Terra")
	record.Set("seed", "0123456789abcdef")
	record.Set("generator_version", 1)
	record.Set("params", map[string]any{"sea_level": 0.4})
	record.Set("owner", owner.Id)
	if err := app.Save(record); err != nil {
		t.Fatal(err)
	}
	return record
}

func authHeader(t testing.TB, record *core.Record) map[string]string {
	t.Helper()
	token, err := record.NewAuthToken()
	if err != nil {
		t.Fatal(err)
	}
	return map[string]string{"Authorization": token}
}

// scenario carries the fixtures a test needs to build its URL and headers
// before the request runs.
type scenario struct {
	tests.ApiScenario
	// setup runs on the fresh app and may fill URL, Headers and Body.
	setup func(t testing.TB, app *tests.TestApp, s *tests.ApiScenario)
}

func (s scenario) run(t *testing.T) {
	t.Helper()
	api := s.ApiScenario
	app := newApp(t)
	if s.setup != nil {
		s.setup(t, app, &api)
	}
	api.TestAppFactory = func(testing.TB) *tests.TestApp { return app }
	api.Test(t)
}

func TestOTPRequestCreatesTheAccount(t *testing.T) {
	scenario{ApiScenario: tests.ApiScenario{
		Name:            "unknown email",
		Method:          http.MethodPost,
		URL:             "/api/collections/users/request-otp",
		Body:            strings.NewReader(`{"email":"New.Person@Example.com"}`),
		Delay:           200 * time.Millisecond, // the email is sent in the background
		ExpectedStatus:  http.StatusOK,
		ExpectedContent: []string{`"otpId":"`},
		AfterTestFunc: func(t testing.TB, app *tests.TestApp, _ *http.Response) {
			user, err := app.FindAuthRecordByEmail(usersCollection, "new.person@example.com")
			if err != nil {
				t.Fatalf("the account was not created: %v", err)
			}
			if user.Email() != "new.person@example.com" {
				t.Errorf("email = %q, want it lower case", user.Email())
			}
			if user.Verified() || user.GetBool("operator") {
				t.Error("a new account must be unverified and not an operator")
			}

			otps, err := app.FindAllOTPsByRecord(user)
			if err != nil || len(otps) != 1 {
				t.Fatalf("otps = %d (%v), want 1", len(otps), err)
			}
			if app.TestMailer.TotalSend() != 1 {
				t.Fatalf("emails sent = %d, want 1", app.TestMailer.TotalSend())
			}
			// The email must carry the same link the log prints, plus a bare code.
			body := html.UnescapeString(app.TestMailer.LastMessage().HTML)
			link := magicLink(testConfig.AppURL, otps[0].Id, "")
			if !strings.Contains(body, link) {
				t.Errorf("email body lacks the magic link %q:\n%s", link, body)
			}
			if !strings.Contains(body, "<strong>") || strings.Contains(body, "{OTP") {
				t.Errorf("email body lacks the plain code or kept a placeholder:\n%s", body)
			}
		},
	}}.run(t)
}

func TestOTPRequestReusesTheAccountWhateverTheCase(t *testing.T) {
	scenario{
		ApiScenario: tests.ApiScenario{
			Name:            "known email, other case",
			Method:          http.MethodPost,
			URL:             "/api/collections/users/request-otp",
			Body:            strings.NewReader(`{"email":"ANA@example.com"}`),
			Delay:           200 * time.Millisecond,
			ExpectedStatus:  http.StatusOK,
			ExpectedContent: []string{`"otpId":"`},
			AfterTestFunc: func(t testing.TB, app *tests.TestApp, _ *http.Response) {
				total, err := app.CountRecords(usersCollection)
				if err != nil || total != 1 {
					t.Errorf("users = %d (%v), want 1", total, err)
				}
			},
		},
		setup: func(t testing.TB, app *tests.TestApp, _ *tests.ApiScenario) {
			createUser(t, app, "ana@example.com", false)
		},
	}.run(t)
}

func TestUsersCannotBeCreatedThroughTheAPI(t *testing.T) {
	scenario{ApiScenario: tests.ApiScenario{
		Name:            "public create",
		Method:          http.MethodPost,
		URL:             "/api/collections/users/records",
		Body:            strings.NewReader(`{"email":"eve@example.com","password":"12345678","passwordConfirm":"12345678","operator":true}`),
		ExpectedStatus:  http.StatusForbidden,
		ExpectedContent: []string{`"data":{}`},
	}}.run(t)
}

func TestPasswordAuthIsOff(t *testing.T) {
	scenario{ApiScenario: tests.ApiScenario{
		Name:            "auth with password",
		Method:          http.MethodPost,
		URL:             "/api/collections/users/auth-with-password",
		Body:            strings.NewReader(`{"identity":"ana@example.com","password":"12345678"}`),
		ExpectedStatus:  http.StatusForbidden,
		ExpectedContent: []string{`"data":{}`},
	}}.run(t)
}

func TestOperatorFlag(t *testing.T) {
	// asUser targets ana's record, as ana herself or as a second account.
	asUser := func(email string, operator bool) func(testing.TB, *tests.TestApp, *tests.ApiScenario) {
		return func(t testing.TB, app *tests.TestApp, s *tests.ApiScenario) {
			ana := createUser(t, app, "ana@example.com", false)
			caller := ana
			if email != ana.Email() {
				caller = createUser(t, app, email, operator)
			}
			s.URL = "/api/collections/users/records/" + ana.Id
			s.Headers = authHeader(t, caller)
		}
	}

	scenarios := []scenario{
		{
			ApiScenario: tests.ApiScenario{
				Name:            "a user cannot grant itself",
				Method:          http.MethodPatch,
				Body:            strings.NewReader(`{"operator":true}`),
				ExpectedStatus:  http.StatusNotFound,
				ExpectedContent: []string{`"data":{}`},
			},
			setup: asUser("ana@example.com", false),
		},
		{
			ApiScenario: tests.ApiScenario{
				Name:            "a user still updates its own profile",
				Method:          http.MethodPatch,
				Body:            strings.NewReader(`{"name":"Ana","operator":false}`),
				ExpectedStatus:  http.StatusOK,
				ExpectedContent: []string{`"name":"Ana"`, `"operator":false`},
			},
			setup: asUser("ana@example.com", false),
		},
		{
			ApiScenario: tests.ApiScenario{
				Name:            "another user cannot touch it",
				Method:          http.MethodPatch,
				Body:            strings.NewReader(`{"name":"Hacked"}`),
				ExpectedStatus:  http.StatusNotFound,
				ExpectedContent: []string{`"data":{}`},
			},
			setup: asUser("bob@example.com", false),
		},
		{
			ApiScenario: tests.ApiScenario{
				Name:            "an operator grants it",
				Method:          http.MethodPatch,
				Body:            strings.NewReader(`{"operator":true}`),
				ExpectedStatus:  http.StatusOK,
				ExpectedContent: []string{`"operator":true`},
			},
			setup: asUser("root@example.com", true),
		},
	}
	for _, s := range scenarios {
		s.run(t)
	}
}

func TestUsersList(t *testing.T) {
	listAs := func(operator bool) func(testing.TB, *tests.TestApp, *tests.ApiScenario) {
		return func(t testing.TB, app *tests.TestApp, s *tests.ApiScenario) {
			createUser(t, app, "ana@example.com", false)
			s.Headers = authHeader(t, createUser(t, app, "caller@example.com", operator))
		}
	}
	scenarios := []scenario{
		{
			ApiScenario: tests.ApiScenario{
				Name:            "a user sees only itself",
				Method:          http.MethodGet,
				URL:             "/api/collections/users/records",
				ExpectedStatus:  http.StatusOK,
				ExpectedContent: []string{`"totalItems":1`},
			},
			setup: listAs(false),
		},
		{
			ApiScenario: tests.ApiScenario{
				Name:            "an operator sees everyone",
				Method:          http.MethodGet,
				URL:             "/api/collections/users/records",
				ExpectedStatus:  http.StatusOK,
				ExpectedContent: []string{`"totalItems":2`},
			},
			setup: listAs(true),
		},
	}
	for _, s := range scenarios {
		s.run(t)
	}
}

func TestWorldsCreate(t *testing.T) {
	// body builds the request once the owner id is known.
	asAna := func(body func(anaID, bobID string) string) func(testing.TB, *tests.TestApp, *tests.ApiScenario) {
		return func(t testing.TB, app *tests.TestApp, s *tests.ApiScenario) {
			ana := createUser(t, app, "ana@example.com", false)
			bob := createUser(t, app, "bob@example.com", false)
			s.Headers = authHeader(t, ana)
			s.Body = strings.NewReader(body(ana.Id, bob.Id))
		}
	}
	create := func(name string, status int, content []string) tests.ApiScenario {
		return tests.ApiScenario{
			Name:            name,
			Method:          http.MethodPost,
			URL:             "/api/collections/worlds/records",
			ExpectedStatus:  status,
			ExpectedContent: content,
		}
	}

	scenarios := []scenario{
		{
			ApiScenario: create("a user creates its own world", http.StatusOK, []string{`"seed":"0123456789abcdef"`, `"generator_version":1`}),
			setup:       asAna(func(ana, _ string) string { return `{` + validWorld + `,"owner":"` + ana + `"}` }),
		},
		{
			ApiScenario: create("not for somebody else", http.StatusBadRequest, []string{`"data":{}`}),
			setup:       asAna(func(_, bob string) string { return `{` + validWorld + `,"owner":"` + bob + `"}` }),
		},
		{
			ApiScenario: create("a visitor cannot create", http.StatusBadRequest, []string{`"data":{}`}),
			setup: func(t testing.TB, app *tests.TestApp, s *tests.ApiScenario) {
				ana := createUser(t, app, "ana@example.com", false)
				s.Body = strings.NewReader(`{` + validWorld + `,"owner":"` + ana.Id + `"}`)
			},
		},
		{
			ApiScenario: create("the seed must be canonical", http.StatusBadRequest, []string{`"seed":{`}),
			setup: asAna(func(ana, _ string) string {
				return `{"name":"Terra","seed":"0123456789ABCDEF","generator_version":1,"owner":"` + ana + `"}`
			}),
		},
		{
			ApiScenario: create("the generator version starts at 1", http.StatusBadRequest, []string{`"generator_version":{`}),
			setup: asAna(func(ana, _ string) string {
				return `{"name":"Terra","seed":"0123456789abcdef","generator_version":0,"owner":"` + ana + `"}`
			}),
		},
		{
			ApiScenario: create("params must be an object", http.StatusBadRequest, []string{`"params":{"code":"validation_invalid_recipe"`}),
			setup: asAna(func(ana, _ string) string {
				return `{"name":"Terra","seed":"0123456789abcdef","generator_version":1,"params":[1],"owner":"` + ana + `"}`
			}),
		},
	}
	for _, s := range scenarios {
		s.run(t)
	}
}

func TestWorldsAreReadableByVisitors(t *testing.T) {
	scenario{
		ApiScenario: tests.ApiScenario{
			Name:            "public list",
			Method:          http.MethodGet,
			URL:             "/api/collections/worlds/records",
			ExpectedStatus:  http.StatusOK,
			ExpectedContent: []string{`"totalItems":1`, `"name":"Terra"`},
		},
		setup: func(t testing.TB, app *tests.TestApp, _ *tests.ApiScenario) {
			createWorld(t, app, createUser(t, app, "ana@example.com", false))
		},
	}.run(t)
}

func TestWorldsUpdate(t *testing.T) {
	// as picks the caller: the owner, a stranger or an operator.
	as := func(caller string) func(testing.TB, *tests.TestApp, *tests.ApiScenario) {
		return func(t testing.TB, app *tests.TestApp, s *tests.ApiScenario) {
			owner := createUser(t, app, "owner@example.com", false)
			callers := map[string]*core.Record{
				"owner":    owner,
				"stranger": createUser(t, app, "stranger@example.com", false),
				"operator": createUser(t, app, "operator@example.com", true),
			}
			s.URL = "/api/collections/worlds/records/" + createWorld(t, app, owner).Id
			s.Headers = authHeader(t, callers[caller])
		}
	}
	patch := func(name, body string, status int, content []string) tests.ApiScenario {
		return tests.ApiScenario{
			Name:            name,
			Method:          http.MethodPatch,
			Body:            strings.NewReader(body),
			ExpectedStatus:  status,
			ExpectedContent: content,
		}
	}
	frozen := []string{`"code":"validation_recipe_frozen"`}

	scenarios := []scenario{
		{patch("the owner renames", `{"name":"Gaia"}`, http.StatusOK, []string{`"name":"Gaia"`}), as("owner")},
		{patch("an operator renames", `{"name":"Gaia"}`, http.StatusOK, []string{`"name":"Gaia"`}), as("operator")},
		{patch("a stranger does not", `{"name":"Gaia"}`, http.StatusNotFound, []string{`"data":{}`}), as("stranger")},
		{patch("the same recipe is no change", `{"name":"Gaia","params":{ "sea_level": 0.4 }}`, http.StatusOK, []string{`"name":"Gaia"`}), as("owner")},
		{patch("the seed is frozen", `{"seed":"ffffffffffffffff"}`, http.StatusBadRequest, frozen), as("owner")},
		{patch("the generator version is frozen", `{"generator_version":2}`, http.StatusBadRequest, frozen), as("owner")},
		{patch("the params are frozen", `{"params":{"sea_level":0.5}}`, http.StatusBadRequest, frozen), as("owner")},
		{patch("frozen for operators too", `{"generator_version":2}`, http.StatusBadRequest, frozen), as("operator")},
	}
	for _, s := range scenarios {
		s.run(t)
	}
}

func TestRecipeIsFrozenBelowTheAPI(t *testing.T) {
	app := newApp(t)
	defer app.Cleanup()

	record := createWorld(t, app, createUser(t, app, "ana@example.com", false))
	record.Set("seed", "ffffffffffffffff")
	if err := app.Save(record); err == nil {
		t.Fatal("a Go side save changed the seed of an existing world")
	}
}

func TestApplySettings(t *testing.T) {
	app := newApp(t)
	defer app.Cleanup()

	withoutMail := testConfig
	withoutMail.SMTP = SMTP{}
	for _, cfg := range []Config{testConfig, testConfig, withoutMail} {
		if err := applySettings(app, cfg); err != nil {
			t.Fatal(err)
		}
		settings := app.Settings()
		if settings.Meta.AppURL != cfg.AppURL || settings.Meta.AppName != cfg.AppName || settings.Meta.SenderAddress != cfg.SenderAddress {
			t.Errorf("meta = %+v, want it to follow %+v", settings.Meta, cfg)
		}
		if settings.SMTP.Enabled != (cfg.SMTP.Host != "") || settings.SMTP.Host != cfg.SMTP.Host {
			t.Errorf("smtp enabled = %v host = %q for config host %q", settings.SMTP.Enabled, settings.SMTP.Host, cfg.SMTP.Host)
		}
	}
}

func TestDefaultConfigIsAccepted(t *testing.T) {
	for _, key := range []string{"APP_URL", "APP_NAME", "SMTP_HOST", "SMTP_PORT", "SMTP_USERNAME", "SMTP_PASSWORD", "SMTP_SENDER", "PLANET_OPERATOR_EMAIL"} {
		t.Setenv(key, "")
	}
	cfg, err := ConfigFromEnv()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.SMTP.Host != "" || cfg.OperatorEmail != "" {
		t.Errorf("mail and operator must be off by default: %+v", cfg)
	}

	app := newApp(t)
	defer app.Cleanup()
	if err := applySettings(app, cfg); err != nil {
		t.Fatalf("PocketBase rejected the default settings: %v", err)
	}
}

func TestEnsureOperator(t *testing.T) {
	app := newApp(t)
	defer app.Cleanup()

	existing := createUser(t, app, "ana@example.com", false)

	// Twice each: the call runs on every start.
	for _, email := range []string{"Ana@Example.com", "Ana@Example.com", "root@example.com", "root@example.com", ""} {
		if err := ensureOperator(app, email); err != nil {
			t.Fatalf("ensureOperator(%q): %v", email, err)
		}
	}

	for _, email := range []string{"ana@example.com", "root@example.com"} {
		user, err := app.FindAuthRecordByEmail(usersCollection, email)
		if err != nil {
			t.Fatalf("%s: %v", email, err)
		}
		if !user.GetBool("operator") {
			t.Errorf("%s is not an operator", email)
		}
	}
	if total, _ := app.CountRecords(usersCollection); total != 2 {
		t.Errorf("users = %d, want 2", total)
	}
	if promoted, _ := app.FindRecordById(usersCollection, existing.Id); promoted == nil || !promoted.GetBool("operator") {
		t.Error("the existing account was not the one promoted")
	}
}

func TestMailIsNotDeliveredWithoutSMTP(t *testing.T) {
	app := newApp(t)
	defer app.Cleanup()

	delivered := func() bool {
		reached := false
		event := &core.MailerEvent{App: app, Message: &mailer.Message{}}
		if err := app.OnMailerSend().Trigger(event, func(*core.MailerEvent) error {
			reached = true
			return nil
		}); err != nil {
			t.Fatal(err)
		}
		return reached
	}

	app.Settings().SMTP.Enabled = true
	if !delivered() {
		t.Error("mail did not reach the transport with SMTP on")
	}
	app.Settings().SMTP.Enabled = false
	if delivered() {
		t.Error("mail reached the transport with SMTP off")
	}
}
