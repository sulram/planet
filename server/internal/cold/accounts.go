package cold

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	"github.com/pocketbase/pocketbase/core"
)

// createAccountOnFirstLogin makes sign up and sign in one flow: an unknown
// email gets an account and a code like any other. The account stays
// unverified until the code proves the inbox; PocketBase flips the flag then.
func createAccountOnFirstLogin(e *core.RecordCreateOTPRequestEvent) error {
	if e.Record != nil {
		return e.Next()
	}

	info, err := e.RequestInfo()
	if err != nil {
		return err
	}
	// PocketBase validated the address before the hook ran.
	email, _ := info.Body["email"].(string)

	record := newUser(e.Collection, email)
	if err := e.App.Save(record); err != nil {
		return fmt.Errorf("create account on first login: %w", err)
	}
	e.Record = record

	return e.Next()
}

// showEmailsToOperators lets an operator read every account's email.
// PocketBase shows an auth record's email to the record itself alone unless
// the record opted in with emailVisibility, which nobody here does, and the
// backoffice lists accounts by email with the operator's own token. Anyone
// else keeps the default: their own address, nobody else's.
func showEmailsToOperators(e *core.RecordEnrichEvent) error {
	if e.RequestInfo != nil && e.RequestInfo.Auth != nil && e.RequestInfo.Auth.GetBool("operator") {
		e.Record.IgnoreEmailVisibility(true)
	}
	return e.Next()
}

// ensureOperator makes sure the named account exists and is an operator. The
// operator signs in by magic link like everyone else.
func ensureOperator(app core.App, email string) error {
	if email == "" {
		return nil
	}

	record, err := app.FindAuthRecordByEmail(usersCollection, normalizeEmail(email))
	if errors.Is(err, sql.ErrNoRows) {
		var users *core.Collection
		if users, err = app.FindCollectionByNameOrId(usersCollection); err == nil {
			record = newUser(users, email)
		}
	}
	if err != nil {
		return fmt.Errorf("ensure operator: %w", err)
	}
	if record.GetBool("operator") {
		return nil
	}

	record.Set("operator", true)
	if err := app.Save(record); err != nil {
		return fmt.Errorf("ensure operator: %w", err)
	}
	return nil
}

// newUser builds an unsaved, unverified user. Password auth is off, so the
// password only satisfies the auth schema: random, never shown to anyone.
func newUser(users *core.Collection, email string) *core.Record {
	record := core.NewRecord(users)
	record.SetEmail(normalizeEmail(email))
	record.SetRandomPassword()
	return record
}

func normalizeEmail(email string) string {
	return strings.ToLower(strings.TrimSpace(email))
}
