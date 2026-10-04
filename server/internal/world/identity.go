package world

import pb "github.com/sulram/planet/server/internal/protocol"

// Level is what a session may do, as mundos says it for one entry. The order
// is the order of trust, and the wire's `Level` counts the same way.
type Level int

const (
	LevelAnonymous Level = iota
	LevelSignedIn
	LevelBuilder
	LevelAdmin
)

var levelNames = [...]string{"anonymous", "signed_in", "builder", "admin"}

// ParseLevel reads a level as mundos writes it. False for any other word.
func ParseLevel(name string) (Level, bool) {
	for level, known := range levelNames {
		if name == known {
			return Level(level), true
		}
	}
	return LevelAnonymous, false
}

func (l Level) String() string {
	if l < 0 || int(l) >= len(levelNames) {
		return levelNames[LevelAnonymous]
	}
	return levelNames[l]
}

func (l Level) wire() pb.Level {
	return pb.Level(l)
}

// Identity is who a session is, as it was settled before the socket opened.
// The world core never asks mundos; it is told.
type Identity struct {
	// The account's id in mundos. Empty for a visitor: someone with no account.
	UserID string
	// The account's name, as mundos signed it. Empty for a visitor.
	Name string
	// What the session may do, for its life.
	Level Level
}

// Visitor is a session without an account.
func (i Identity) Visitor() bool {
	return i.UserID == ""
}
