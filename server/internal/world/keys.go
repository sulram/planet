package world

import (
	"crypto/rand"
	"encoding/base64"
	"sync"
	"time"
)

// KeyLife is how long a key stands: a long day's visit. A page that outlives
// it enters again through mundos's door.
const KeyLife = 12 * time.Hour

// Keys holds who entered: an identity settled at the door, under a random key
// the page keeps in memory and shows on every socket it opens and on every
// route that changes the world. A browser cannot put a header on a WebSocket
// and mundos's token lives a minute, so the key is what a page comes back with
// when a link drops. It lives as long as the page does: a reload passes
// through mundos again, and the level is read anew.
type Keys struct {
	now func() time.Time
	mu  sync.Mutex
	// Key to identity and expiry.
	held map[string]held
}

type held struct {
	identity Identity
	expires  time.Time
}

func NewKeys() *Keys {
	return newKeys(time.Now)
}

func newKeys(now func() time.Time) *Keys {
	return &Keys{now: now, held: map[string]held{}}
}

// Mint gives an identity its key. Expired keys are swept here, so the map
// never grows past what a day of entries leaves.
func (k *Keys) Mint(identity Identity) string {
	raw := make([]byte, 24)
	if _, err := rand.Read(raw); err != nil {
		panic("crypto/rand: " + err.Error())
	}
	key := base64.RawURLEncoding.EncodeToString(raw)

	k.mu.Lock()
	defer k.mu.Unlock()
	now := k.now()
	for old, entry := range k.held {
		if !entry.expires.After(now) {
			delete(k.held, old)
		}
	}
	k.held[key] = held{identity: identity, expires: now.Add(KeyLife)}
	return key
}

// Find is whose key this is. False when it is unknown or expired.
func (k *Keys) Find(key string) (Identity, bool) {
	k.mu.Lock()
	defer k.mu.Unlock()
	entry, ok := k.held[key]
	if !ok || !entry.expires.After(k.now()) {
		return Identity{}, false
	}
	return entry.identity, true
}
