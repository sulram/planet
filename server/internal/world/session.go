package world

import (
	"context"
	"sync"
	"time"

	"google.golang.org/protobuf/proto"

	pb "github.com/sulram/planet/server/internal/protocol"
)

// outboxDepth is how many frames a client may fall behind before it is
// dropped as too slow to follow: a second of ticks.
const outboxDepth = 32

// session is one connection inside a world. The actor owns its fields; the
// reader and writer goroutines touch only the channels.
type session struct {
	id       uint32
	identity Identity
	avatar   string
	stance   *pb.Stance
	conn     Conn
	// When the last lineBurst lines were said, oldest first, for the rate
	// limit. Touched by the actor only.
	said [lineBurst]time.Time
	// Frames for the writer. Closed by the actor when the session ends.
	out chan []byte
	// Closed once, when the actor lets the session go.
	ended chan struct{}
	once  sync.Once
}

func newSession(identity Identity, avatar string, conn Conn) *session {
	return &session{
		identity: identity,
		avatar:   avatar,
		conn:     conn,
		out:      make(chan []byte, outboxDepth),
		ended:    make(chan struct{}),
	}
}

func (s *session) peer() *pb.Peer {
	return &pb.Peer{
		Session: s.id,
		Name:    s.identity.Name,
		Visitor: s.identity.Visitor(),
		Avatar:  s.avatar,
		Stance:  s.stance,
	}
}

// mayspeak is the rate limit: true, and the line counted, unless lineBurst
// lines were already said within lineWindow.
func (s *session) mayspeak(now time.Time) bool {
	if now.Sub(s.said[0]) < lineWindow {
		return false
	}
	copy(s.said[:], s.said[1:])
	s.said[lineBurst-1] = now
	return true
}

// send queues a message for the writer, dropping the session if it is behind.
func (s *session) send(message *pb.ServerMessage) {
	if frame, err := proto.Marshal(message); err == nil {
		s.offer(frame)
	}
}

// offer queues a frame without waiting. False when the client is behind.
func (s *session) offer(frame []byte) bool {
	select {
	case <-s.ended:
		return false
	default:
	}
	select {
	case s.out <- frame:
		return true
	default:
		return false
	}
}

// end is the actor letting go: the writer drains what is queued and the
// reader stops feeding the inbox.
func (s *session) end() {
	s.once.Do(func() {
		close(s.ended)
		close(s.out)
	})
}

// run pumps the connection until it ends: frames in to the actor's inbox,
// frames out from the outbox. Returns when the actor has let the session go.
func (s *session) run(ctx context.Context, a *actor) error {
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()

	go func() {
		for frame := range s.out {
			if err := s.conn.Write(ctx, frame); err != nil {
				cancel()
				return
			}
		}
	}()

	var failure error
	for {
		frame, err := s.read(ctx)
		if err != nil {
			failure = err
			break
		}
		var message pb.ClientMessage
		if err := proto.Unmarshal(frame, &message); err != nil {
			failure = err
			break
		}
		select {
		case a.inbox <- inbound{kind: messageKind, session: s, message: &message}:
		case <-s.ended:
		}
		if s.gone() {
			break
		}
	}

	// Either the link died or the actor dropped us. Say goodbye in both
	// cases; the actor ignores a leave for a session it no longer holds.
	select {
	case a.inbox <- inbound{kind: leaveKind, session: s}:
	case <-s.ended:
	}
	<-s.ended
	cancel()
	return failure
}

func (s *session) read(ctx context.Context) ([]byte, error) {
	ctx, cancel := context.WithTimeout(ctx, readTimeout)
	defer cancel()
	return s.conn.Read(ctx)
}

func (s *session) gone() bool {
	select {
	case <-s.ended:
		return true
	default:
		return false
	}
}
