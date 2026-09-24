package world

// Identity is who a session is, as the cold plane established it before the
// socket opened. The hot plane never asks PocketBase; it is told.
type Identity struct {
	// Empty for a visitor: someone signed out, who looks and never builds.
	UserID string
	// As the person set it. Empty when they set none.
	Name string
}

// Visitor is a session without an account.
func (i Identity) Visitor() bool {
	return i.UserID == ""
}
