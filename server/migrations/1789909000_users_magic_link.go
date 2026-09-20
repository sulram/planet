package migrations

import (
	"errors"

	"github.com/pocketbase/pocketbase/core"
	m "github.com/pocketbase/pocketbase/migrations"
	"github.com/pocketbase/pocketbase/tools/dbutils"
	"github.com/pocketbase/pocketbase/tools/types"
)

// The link serves the browser, the bare code serves native clients and
// headsets, where the user types it. One email carries both.
const otpEmailBody = `<p>Hello,</p>
<p><a href="{APP_URL}/login/verify?otpId={OTP_ID}&amp;code={OTP}">Sign in to {APP_NAME}</a></p>
<p>Signing in on another device? Type this code there: <strong>{OTP}</strong></p>
<p>The link and the code work once and expire in a few minutes. If you did not ask for them, ignore this email.</p>`

func init() {
	m.Register(func(app core.App) error {
		users, err := app.FindCollectionByNameOrId("users")
		if err != nil {
			return err
		}

		users.Fields.Add(&core.BoolField{Name: "operator"})

		users.OTP.Enabled = true
		users.OTP.EmailTemplate.Subject = "Sign in to {APP_NAME}"
		users.OTP.EmailTemplate.Body = otpEmailBody
		users.PasswordAuth.Enabled = false
		// Every sign in already goes through the inbox, so a "new login"
		// alert would only be a second email about the first.
		users.AuthAlert.Enabled = false

		// Addresses differing only by case are the same account.
		index, ok := dbutils.FindSingleColumnUniqueIndex(users.Indexes, core.FieldNameEmail)
		if !ok {
			return errors.New("users has no unique email index")
		}
		users.AddIndex(index.IndexName, true, "`email` COLLATE NOCASE", "`email` != ''")

		self := "id = @request.auth.id"
		operator := "@request.auth.operator = true"
		users.ListRule = types.Pointer(self + " || " + operator)
		users.ViewRule = types.Pointer(self + " || " + operator)
		// Accounts are born from the first OTP request only. An open create
		// endpoint would let anyone post operator = true.
		users.CreateRule = nil
		users.UpdateRule = types.Pointer("(" + self + " && @request.body.operator:changed = false) || " + operator)
		// Deleting an account needs a decision about the worlds it owns.
		users.DeleteRule = nil

		return app.Save(users)
	}, func(app core.App) error {
		users, err := app.FindCollectionByNameOrId("users")
		if err != nil {
			return err
		}

		users.Fields.RemoveByName("operator")
		// The template stays: it is inert while OTP is off.
		users.OTP.Enabled = false
		users.PasswordAuth.Enabled = true
		users.AuthAlert.Enabled = true

		if index, ok := dbutils.FindSingleColumnUniqueIndex(users.Indexes, core.FieldNameEmail); ok {
			users.AddIndex(index.IndexName, true, "`email`", "`email` != ''")
		}

		self := "id = @request.auth.id"
		users.ListRule = types.Pointer(self)
		users.ViewRule = types.Pointer(self)
		users.CreateRule = types.Pointer("")
		users.UpdateRule = types.Pointer(self)
		users.DeleteRule = types.Pointer(self)

		return app.Save(users)
	})
}
