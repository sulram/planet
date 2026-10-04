// Package api is the instance's HTTP face: the routes a page asks, the world
// socket and the front end's files. It is the one package that knows HTTP,
// mundos's token and the world folder at once; the world core behind it is
// told who a session is and never asks.
package api

import (
	"crypto/ed25519"
	"encoding/json"
	"errors"
	"io"
	"log"
	"net/http"
	"strings"
	"time"

	"github.com/sulram/planet/server/internal/folder"
	"github.com/sulram/planet/server/internal/mundos"
	"github.com/sulram/planet/server/internal/world"
)

// Config is what the environment decides about an instance.
type Config struct {
	// Name is the world's name in mundos, the audience of its tokens.
	Name string
	// MundosURL is mundos's origin, where the door is.
	MundosURL string
	// PublicKey checks mundos's tokens. Nil for a world that runs alone, where
	// every session has DevLevel.
	PublicKey ed25519.PublicKey
	// DevLevel is every session's level in a world that runs alone. Under
	// mundos it is anonymous: mundos says the level.
	DevLevel world.Level
	// WebDir holds the front end's files. Empty in development, where Vite
	// serves them and proxies the routes here.
	WebDir string
	// Version is the engine's, as the build names it.
	Version string
}

// Server is one instance: one world.
type Server struct {
	cfg    Config
	folder *folder.Folder
	hub    *world.Hub
	keys   *world.Keys
	now    func() time.Time
}

func New(cfg Config, f *folder.Folder) *Server {
	return &Server{cfg: cfg, folder: f, hub: world.NewHub(f), keys: world.NewKeys(), now: time.Now}
}

// Handler is every route of the instance. A path under /api that is no route
// is a 404 and never the page.
func (s *Server) Handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /api/health", s.health)
	mux.HandleFunc("GET /api/world", s.describe)
	mux.HandleFunc("POST /api/world", s.found)
	mux.HandleFunc("POST /api/enter", s.enter)
	mux.HandleFunc("GET /api/me", s.me)
	mux.HandleFunc("GET /api/socket", s.socket)
	mux.Handle("/api/", http.NotFoundHandler())
	mux.Handle("/", files(s.cfg.WebDir))
	return mux
}

// What a body may weigh: a recipe's params are a handful of knobs.
const maxBody = 32 << 10

func (s *Server) health(w http.ResponseWriter, _ *http.Request) {
	w.Header().Set("Cache-Control", "no-store")
	_, _ = io.WriteString(w, "ok\n")
}

// description is what a world says it is, to a page and to mundos.
type description struct {
	Name    string `json:"name"`
	Founded bool   `json:"founded"`
	// Null while the world is unfounded.
	Recipe *recipe `json:"recipe"`
	// What it speaks: the wire's version and the engine's.
	Protocol int    `json:"protocol"`
	Version  string `json:"version"`
	// mundos's door, which every load passes through. Empty for a world that
	// runs alone.
	Door string `json:"door"`
}

// recipe has the wire's shape, as the page and the engine read it.
type recipe struct {
	Seed             string          `json:"seed"`
	GeneratorVersion int             `json:"generator_version"`
	Params           json.RawMessage `json:"params"`
}

func (s *Server) describe(w http.ResponseWriter, r *http.Request) {
	about := description{Name: s.cfg.Name, Protocol: world.Protocol, Version: s.cfg.Version}
	if s.cfg.MundosURL != "" {
		about.Door = s.cfg.MundosURL + "/enter"
	}
	held, err := s.folder.Recipe(r.Context())
	switch {
	case err == nil:
		about.Founded = true
		about.Recipe = &recipe{Seed: held.Seed.String(), GeneratorVersion: held.GeneratorVersion, Params: held.Params}
	case !errors.Is(err, world.ErrUnfounded):
		fail(w, http.StatusInternalServerError, "unreadable")
		return
	}
	answer(w, http.StatusOK, about)
}

// entered is who the server takes a page's visitor for.
type entered struct {
	Level string `json:"level"`
	// The account's name, as mundos signed it. Empty for a visitor.
	Name string `json:"name"`
	// What the page shows on the socket and on a founding. Empty for a
	// visitor, who enters as a guest.
	Key string `json:"key"`
}

// enter trades what the door gave the page, a token or `guest`, for who the
// page is here. A token that does not check is a visitor: the door is where
// a person signs in, never this route.
func (s *Server) enter(w http.ResponseWriter, r *http.Request) {
	var asked struct {
		Identity string `json:"identity"`
	}
	if !read(w, r, &asked) {
		return
	}
	identity := world.Identity{Level: s.cfg.DevLevel}
	if s.cfg.PublicKey != nil && asked.Identity != "" && asked.Identity != "guest" {
		claims, err := mundos.Verify(asked.Identity, s.cfg.PublicKey, s.cfg.Name, s.now())
		level, known := world.ParseLevel(claims.Level)
		if err == nil && known {
			identity = world.Identity{UserID: claims.Sub, Name: claims.Name, Level: level}
		} else {
			log.Print("enter: a token was refused, entering as a visitor")
		}
	}
	who := entered{Level: identity.Level.String(), Name: identity.Name}
	if !identity.Visitor() {
		who.Key = s.keys.Mint(identity)
	}
	answer(w, http.StatusOK, who)
}

// me says whether a key still stands. A page asks before it opens a link
// again: a key is forgotten when the server restarts and when its life ends,
// and the page then passes through the door for another.
func (s *Server) me(w http.ResponseWriter, r *http.Request) {
	key := bearer(r)
	identity, known := s.who(key)
	if !known {
		fail(w, http.StatusUnauthorized, "key")
		return
	}
	answer(w, http.StatusOK, entered{Level: identity.Level.String(), Name: identity.Name, Key: key})
}

// bearer is the key a request carries, or nothing.
func bearer(r *http.Request) string {
	key, _ := strings.CutPrefix(r.Header.Get("Authorization"), "Bearer ")
	return key
}

// who is the identity a key stands for. A request with an empty key is a
// visitor's, at the level every session has in a world that runs alone. Only a
// key this server gave is true.
func (s *Server) who(key string) (world.Identity, bool) {
	if key == "" {
		return world.Identity{Level: s.cfg.DevLevel}, true
	}
	return s.keys.Find(key)
}

// found freezes the recipe of an unfounded world. An admin's alone.
func (s *Server) found(w http.ResponseWriter, r *http.Request) {
	identity, known := s.who(bearer(r))
	if !known {
		fail(w, http.StatusUnauthorized, "key")
		return
	}
	if identity.Level != world.LevelAdmin {
		fail(w, http.StatusForbidden, "level")
		return
	}
	var asked recipe
	if !read(w, r, &asked) {
		return
	}
	founded, err := world.NewRecipe(asked.Seed, asked.GeneratorVersion, asked.Params)
	switch {
	case errors.Is(err, world.ErrSeed):
		fail(w, http.StatusBadRequest, "seed")
		return
	case errors.Is(err, world.ErrGeneratorVersion):
		fail(w, http.StatusBadRequest, "generator_version")
		return
	case err != nil:
		fail(w, http.StatusBadRequest, "params")
		return
	}
	switch err := s.folder.Found(founded); {
	case errors.Is(err, folder.ErrFounded):
		fail(w, http.StatusConflict, "founded")
		return
	case err != nil:
		log.Printf("found: %v", err)
		fail(w, http.StatusInternalServerError, "unwritable")
		return
	}
	log.Printf("the world is founded by %q: seed %s, generator %d", identity.UserID, founded.Seed, founded.GeneratorVersion)
	s.describe(w, r)
}

// read decodes a JSON body of a sane size, or answers 400 and says false.
func read(w http.ResponseWriter, r *http.Request, into any) bool {
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, maxBody)).Decode(into); err != nil {
		fail(w, http.StatusBadRequest, "body")
		return false
	}
	return true
}

func answer(w http.ResponseWriter, status int, body any) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Cache-Control", "no-store")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(body)
}

// fail answers with a code a front end matches, never a sentence: the page
// owns the words.
func fail(w http.ResponseWriter, status int, code string) {
	answer(w, status, map[string]string{"error": code})
}
