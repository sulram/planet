package world

import "context"

// Conn is one client's connection as the world sees it: binary frames in,
// binary frames out, each frame one message. The WebSocket behind it lives
// in the routes' glue (`internal/api`); tests use a pair of channels.
type Conn interface {
	// Read blocks for the next frame. An error ends the session.
	Read(ctx context.Context) ([]byte, error)
	Write(ctx context.Context, frame []byte) error
	// Close ends the connection with a reason the client may log.
	Close(reason string) error
}
