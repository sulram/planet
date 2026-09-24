package cold

import (
	"context"
	"database/sql"
	"errors"

	"github.com/coder/websocket"
	"github.com/pocketbase/pocketbase/apis"
	"github.com/pocketbase/pocketbase/core"

	"github.com/sulram/planet/server/internal/world"
)

// The hot plane's two doors, in the cold plane's router. Everything behind
// them is internal/world, which never sees a PocketBase type: identity is
// settled here and handed over, and the recipe is read here on request.

// registerHot binds the ticket and socket routes.
func registerHot(e *core.ServeEvent, hub *world.Hub, tickets *world.Tickets) {
	e.Router.POST("/api/planet/ticket", mintTicket(tickets)).Bind(apis.RequireAuth(usersCollection))
	e.Router.GET("/api/planet/worlds/{id}/socket", openSocket(hub, tickets))
}

// mintTicket gives a signed in person a ticket for the socket. The web app
// asks with the person's own token, from its server side, so the session
// cookie never has to reach a script.
func mintTicket(tickets *world.Tickets) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		identity := world.Identity{
			UserID: e.Auth.Id,
			Name:   e.Auth.GetString("name"),
		}
		return e.JSON(200, map[string]string{"ticket": tickets.Mint(identity)})
	}
}

// openSocket turns the request into a world session and runs it to the end.
// A ticket in the query names the person; no ticket is a visitor.
func openSocket(hub *world.Hub, tickets *world.Tickets) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		identity := world.Identity{}
		if token := e.Request.URL.Query().Get("ticket"); token != "" {
			who, ok := tickets.Redeem(token)
			if !ok {
				return e.UnauthorizedError("The ticket is not valid.", nil)
			}
			identity = who
		}

		conn, err := websocket.Accept(e.Response, e.Request, &websocket.AcceptOptions{
			// The only credential on this socket is the ticket, minted a moment
			// ago for this person and spent on arrival. No cookie rides it, so
			// there is nothing an origin check would protect.
			InsecureSkipVerify: true,
		})
		if err != nil {
			// Accept has answered the request itself.
			return nil
		}

		err = hub.Join(e.Request.Context(), e.Request.PathValue("id"), identity, socketConn{conn})
		if errors.Is(err, world.ErrRefused) {
			return nil
		}
		_ = conn.Close(websocket.StatusNormalClosure, "")
		return nil
	}
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

// catalog is the world core's view of the worlds collection.
type catalog struct {
	app core.App
}

func (c catalog) Recipe(_ context.Context, worldID string) (world.Recipe, error) {
	record, err := c.app.FindRecordById(worldsCollection, worldID)
	if errors.Is(err, sql.ErrNoRows) {
		return world.Recipe{}, world.ErrNoWorld
	}
	if err != nil {
		return world.Recipe{}, err
	}
	return recipeOf(record)
}
