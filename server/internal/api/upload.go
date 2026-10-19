package api

import (
	"errors"
	"io"
	"log"
	"net/http"
	"os"

	"github.com/sulram/planet/server/internal/files"
	"github.com/sulram/planet/server/internal/world"
)

// filesDir is where the world folder keeps its files, when no bucket holds
// them (DECISIONS 89).
const filesDir = "files"

// filesRoute is where a page reads them from, and what `asset://` stands for.
const filesRoute = "/files"

// asset is what a kept file answers with: the name a record keeps.
type asset struct {
	Asset string `json:"asset"`
	Bytes int64  `json:"bytes"`
}

func named(kept files.Kept) asset {
	return asset{Asset: "asset://" + kept.Name, Bytes: kept.Bytes}
}

// upload takes a file from a session that may build: the body as it is, its
// kind read off its bytes. 201 with the asset's name, 200 when the world
// held it already.
func (s *Server) upload(w http.ResponseWriter, r *http.Request) {
	identity, known := s.who(bearer(r))
	if !known {
		fail(w, http.StatusUnauthorized, "key")
		return
	}
	if identity.Level < world.LevelBuilder {
		fail(w, http.StatusForbidden, "level")
		return
	}
	content, err := io.ReadAll(http.MaxBytesReader(w, r.Body, files.Cap))
	var tooLarge *http.MaxBytesError
	switch {
	case errors.As(err, &tooLarge):
		fail(w, http.StatusRequestEntityTooLarge, "size")
		return
	case err != nil:
		fail(w, http.StatusBadRequest, "body")
		return
	}
	kept, held, err := s.files.Keep(content)
	switch {
	case errors.Is(err, files.ErrKind):
		fail(w, http.StatusUnsupportedMediaType, "kind")
		return
	case err != nil:
		log.Printf("files: %v", err)
		fail(w, http.StatusInternalServerError, "unwritable")
		return
	}
	status := http.StatusCreated
	if held {
		status = http.StatusOK
	} else {
		log.Printf("the file %s is kept, %d bytes, by %q", kept.Name, kept.Bytes, identity.UserID)
	}
	answer(w, status, named(kept))
}

// held lists the files the world holds, to a session that may build.
func (s *Server) held(w http.ResponseWriter, r *http.Request) {
	identity, known := s.who(bearer(r))
	if !known {
		fail(w, http.StatusUnauthorized, "key")
		return
	}
	if identity.Level < world.LevelBuilder {
		fail(w, http.StatusForbidden, "level")
		return
	}
	kept, err := s.files.List()
	if err != nil {
		log.Printf("files: %v", err)
		fail(w, http.StatusInternalServerError, "unreadable")
		return
	}
	assets := make([]asset, 0, len(kept))
	for _, each := range kept {
		assets = append(assets, named(each))
	}
	answer(w, http.StatusOK, assets)
}

// file serves one file the world holds. Its name is its content, so a
// browser keeps it for good.
func (s *Server) file(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")
	full, ok := s.files.Path(name)
	if !ok {
		http.NotFound(w, r)
		return
	}
	file, err := os.Open(full)
	if err != nil {
		http.NotFound(w, r)
		return
	}
	defer file.Close()
	info, err := file.Stat()
	if err != nil {
		http.NotFound(w, r)
		return
	}
	header := w.Header()
	header.Set("Content-Type", contentType(name))
	header.Set("Cache-Control", forGood)
	http.ServeContent(w, r, "", info.ModTime(), file)
}
