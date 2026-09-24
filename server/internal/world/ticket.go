package world

import (
	"crypto/rand"
	"encoding/base64"
	"sync"
	"time"
)

// TicketLife is how long a ticket may wait to be redeemed. A page mints one
// right before it connects, so a minute is generous.
const TicketLife = time.Minute

// Tickets turns an identity the cold plane has checked into a short lived,
// single use token the socket can carry. A browser cannot put an httpOnly
// session on a WebSocket, and the hot plane must never read PocketBase, so
// the ticket is the one thing that crosses.
type Tickets struct {
	now func() time.Time
	mu  sync.Mutex
	// Token to identity and expiry.
	open map[string]ticket
}

type ticket struct {
	identity Identity
	expires  time.Time
}

func NewTickets() *Tickets {
	return newTickets(time.Now)
}

func newTickets(now func() time.Time) *Tickets {
	return &Tickets{now: now, open: map[string]ticket{}}
}

// Mint issues a ticket for an identity. Expired tickets are swept here, so
// the map never grows past what a minute of sign ins leaves.
func (t *Tickets) Mint(identity Identity) string {
	raw := make([]byte, 24)
	if _, err := rand.Read(raw); err != nil {
		panic("crypto/rand: " + err.Error())
	}
	token := base64.RawURLEncoding.EncodeToString(raw)

	t.mu.Lock()
	defer t.mu.Unlock()
	now := t.now()
	for old, held := range t.open {
		if !held.expires.After(now) {
			delete(t.open, old)
		}
	}
	t.open[token] = ticket{identity: identity, expires: now.Add(TicketLife)}
	return token
}

// Redeem takes a ticket back. False when it is unknown, used or expired.
func (t *Tickets) Redeem(token string) (Identity, bool) {
	t.mu.Lock()
	defer t.mu.Unlock()
	held, ok := t.open[token]
	if !ok {
		return Identity{}, false
	}
	delete(t.open, token)
	if !held.expires.After(t.now()) {
		return Identity{}, false
	}
	return held.identity, true
}
