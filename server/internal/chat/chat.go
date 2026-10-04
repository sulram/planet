// Package chat is the chat plugin's server half (DECISIONS 69): a line said
// is relayed to everyone in its scope and never stored. It owns no state of
// the world, only how often each session spoke lately.
package chat

import (
	"strings"
	"time"
	"unicode/utf8"

	"google.golang.org/protobuf/proto"

	"github.com/sulram/planet/server/internal/chat/wire"
	"github.com/sulram/planet/server/internal/world"
)

// Name is what the envelope and the world's statement call the plugin.
const Name = "chat"

// Version is that of the plugin's wire and of its seam.
const Version = 1

// The op a session asks, and the event everyone in scope hears.
const (
	opSay     = "say"
	eventSaid = "said"
)

// A line is at most this many characters, counted as a person counts them
// (code points, never bytes), and a session says at most lineBurst lines in
// lineWindow. Past either the line is dropped and nobody is told: a UI holds
// the same limits, so a person never meets them, and only a client that
// ignores them does. The wire's worst case is four bytes a character.
const (
	LineChars  = 500
	lineBurst  = 5
	lineWindow = 5 * time.Second
)

// NearBlocks is how far a line said near reaches: the distance between two
// stances on the same body, in blocks, along the datum and up. One constant
// for every world until a world asks for its own.
const NearBlocks = 64.0

// Chat is the plugin. The actor calls it, one call at a time.
type Chat struct {
	// When each session said its last lineBurst lines, oldest first.
	said map[uint32]*[lineBurst]time.Time
}

func New() *Chat {
	return &Chat{said: map[uint32]*[lineBurst]time.Time{}}
}

func (c *Chat) Name() string    { return Name }
func (c *Chat) Version() uint32 { return Version }

// Ops: anyone in the world may speak, a visitor included. A mute is an
// answer over the permission hook.
func (c *Chat) Ops() []world.Op {
	return []world.Op{{Kind: opSay, Level: world.LevelAnonymous}}
}

// Do relays a line to everyone in its scope, the speaker included, so what
// a client shows is what the world heard. The speaker's place rides along
// when asked for, from the stance the actor holds and never from the
// client's word.
func (c *Chat) Do(room world.Room, who world.Who, kind string, payload []byte) {
	if kind != opSay {
		return
	}
	var say wire.Say
	if err := proto.Unmarshal(payload, &say); err != nil {
		return
	}
	text := strings.TrimSpace(say.Text)
	if !utf8.ValidString(text) || utf8.RuneCountInString(text) > LineChars || (text == "" && !say.Here) {
		return
	}
	if !c.mayspeak(who.Session, room.Now()) {
		return
	}
	said := &wire.Said{Session: who.Session, Scope: say.Scope, Text: text}
	if say.Here {
		said.Stance = who.Stance
	}
	line, err := proto.Marshal(said)
	if err != nil {
		return
	}
	room.Tell(eventSaid, line, func(other world.Who) bool {
		return say.Scope == wire.Scope_SCOPE_WORLD || other.Session == who.Session || room.Near(who, other, NearBlocks)
	})
}

// Gone forgets how often a session spoke.
func (c *Chat) Gone(session uint32) {
	delete(c.said, session)
}

// mayspeak is the rate limit: true, and the line counted, unless lineBurst
// lines were already said within lineWindow.
func (c *Chat) mayspeak(session uint32, now time.Time) bool {
	said := c.said[session]
	if said == nil {
		said = &[lineBurst]time.Time{}
		c.said[session] = said
	}
	if now.Sub(said[0]) < lineWindow {
		return false
	}
	copy(said[:], said[1:])
	said[lineBurst-1] = now
	return true
}
