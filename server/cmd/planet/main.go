// Command planet is one instance: one world. It serves the world socket, the
// routes a page asks and the front end's files, and keeps the world in its
// folder. mundos starts it, with who it is in the environment.
package main

import (
	"context"
	"errors"
	"fmt"
	"log"
	"net"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"github.com/sulram/planet/server/internal/api"
	"github.com/sulram/planet/server/internal/folder"
	"github.com/sulram/planet/server/internal/mundos"
	"github.com/sulram/planet/server/internal/world"
)

// version is the engine's, set by the build (`-ldflags -X main.version=`).
var version = "dev"

func main() {
	command := "serve"
	if len(os.Args) > 1 {
		command = os.Args[1]
	}
	switch command {
	case "serve":
		if err := serve(); err != nil {
			log.Fatal(err)
		}
	case "copy":
		// What mundos runs, from the source's own image, to fill the folder of
		// a world's next generation before it first starts.
		if len(os.Args) != 4 {
			fmt.Fprintln(os.Stderr, "usage: planet copy <from> <to>")
			os.Exit(2)
		}
		if err := folder.Copy(os.Args[2], os.Args[3]); err != nil {
			log.Fatal(err)
		}
	case "version":
		fmt.Println(version)
	default:
		fmt.Fprintln(os.Stderr, "usage: planet [serve | copy <from> <to> | version]")
		os.Exit(2)
	}
}

func serve() error {
	cfg, err := configFromEnv()
	if err != nil {
		return err
	}
	kept, err := folder.Open(envOr("WORLD_DIR", "world"))
	if err != nil {
		return err
	}

	server := &http.Server{
		Addr:              net.JoinHostPort(envOr("HOST", "127.0.0.1"), envOr("PORT", "8090")),
		Handler:           api.New(cfg, kept).Handler(),
		ReadHeaderTimeout: 10 * time.Second,
	}
	// mundos stops a world with SIGTERM: stop listening, let the requests in
	// flight answer, and go. The sockets end with the process.
	stop, cancel := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer cancel()
	go func() {
		<-stop.Done()
		wait, done := context.WithTimeout(context.Background(), 5*time.Second)
		defer done()
		_ = server.Shutdown(wait)
	}()

	if cfg.PublicKey == nil {
		log.Printf("planet %s on %s, alone: every session is %s", version, server.Addr, cfg.DevLevel)
	} else {
		log.Printf("planet %s on %s, the world %q of %s", version, server.Addr, cfg.Name, cfg.MundosURL)
	}
	if err := server.ListenAndServe(); !errors.Is(err, http.ErrServerClosed) {
		return err
	}
	return nil
}

// configFromEnv reads who the instance is. The three variables of mundos
// come together or not at all: with them, people arrive signed; without
// them, the world runs alone and every session has PLANET_DEV_LEVEL.
func configFromEnv() (api.Config, error) {
	cfg := api.Config{
		Name:      os.Getenv("PUBLIC_MUNDOS_WORLD"),
		MundosURL: strings.TrimRight(os.Getenv("PUBLIC_MUNDOS_URL"), "/"),
		WebDir:    os.Getenv("WEB_DIR"),
		Version:   version,
	}
	key := os.Getenv("MUNDOS_PUBLIC_KEY")
	dev := os.Getenv("PLANET_DEV_LEVEL")

	if key == "" && cfg.Name == "" && cfg.MundosURL == "" {
		if dev != "" {
			level, known := world.ParseLevel(dev)
			if !known {
				return cfg, fmt.Errorf("PLANET_DEV_LEVEL: %q is no level (anonymous, signed_in, builder, admin)", dev)
			}
			cfg.DevLevel = level
		}
		return cfg, nil
	}
	if key == "" || cfg.Name == "" || cfg.MundosURL == "" {
		return cfg, errors.New("MUNDOS_PUBLIC_KEY, PUBLIC_MUNDOS_URL and PUBLIC_MUNDOS_WORLD are set together or not at all")
	}
	// A level handed out by a variable, beside mundos's, would be a second
	// answer to who someone is.
	if dev != "" {
		return cfg, errors.New("PLANET_DEV_LEVEL has no place beside MUNDOS_PUBLIC_KEY: mundos says the level")
	}
	public, err := mundos.ParsePublicKey(key)
	if err != nil {
		return cfg, fmt.Errorf("MUNDOS_PUBLIC_KEY: %w", err)
	}
	cfg.PublicKey = public
	return cfg, nil
}

// envOr treats an empty variable as unset, which is how a blank line in .env
// reads.
func envOr(key, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}
