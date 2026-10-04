package api

import (
	"context"
	"errors"
	"net/http"

	"github.com/coder/websocket"

	"github.com/sulram/planet/server/internal/world"
)

// socket turns the request into a session of the world and runs it to the
// end. The key in the query names who it is; the bare route is a visitor's.
func (s *Server) socket(w http.ResponseWriter, r *http.Request) {
	identity, known := s.who(r.URL.Query().Get("key"))
	if !known {
		// The page hears this as a link that never opened, and enters again
		// through the door for a key that stands.
		fail(w, http.StatusUnauthorized, "key")
		return
	}

	conn, err := websocket.Accept(w, r, &websocket.AcceptOptions{
		// The only credential on this socket is the key, given to this page
		// when it entered. No cookie rides it, so there is nothing an origin
		// check would protect.
		InsecureSkipVerify: true,
	})
	if err != nil {
		// Accept has answered the request itself.
		return
	}

	err = s.hub.Join(r.Context(), identity, socketConn{conn})
	if errors.Is(err, world.ErrRefused) {
		return
	}
	_ = conn.Close(websocket.StatusNormalClosure, "")
}

// socketConn is a WebSocket as the world core reads it: binary frames only.
type socketConn struct {
	*websocket.Conn
}

func (c socketConn) Read(ctx context.Context) ([]byte, error) {
	kind, frame, err := c.Conn.Read(ctx)
	if err != nil {
		return nil, err
	}
	if kind != websocket.MessageBinary {
		return nil, errors.New("a text frame on a binary protocol")
	}
	return frame, nil
}

func (c socketConn) Write(ctx context.Context, frame []byte) error {
	return c.Conn.Write(ctx, websocket.MessageBinary, frame)
}

func (c socketConn) Close(reason string) error {
	return c.Conn.Close(websocket.StatusPolicyViolation, reason)
}
