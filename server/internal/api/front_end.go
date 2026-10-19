package api

import (
	"mime"
	"net/http"
	"os"
	"path"
	"path/filepath"
	"strings"
)

// forGood is the cache of what never changes under its URL.
const forGood = "public, max-age=31536000, immutable"

// files serves the front end: the page built into plain files, and the asset
// set beside it. In development the folder is unset: Vite holds the page and
// proxies the routes here.
func files(dir string) http.Handler {
	if dir == "" {
		return http.NotFoundHandler()
	}
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet && r.Method != http.MethodHead {
			w.Header().Set("Allow", "GET, HEAD")
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		// Rooted and cleaned, so no path climbs out of the folder.
		name := path.Clean("/" + r.URL.Path)
		// A route is a file by its own name: `/ds` is `ds.html`, `/` is `index.html`.
		for _, candidate := range []string{name, name + ".html", path.Join(name, "index.html")} {
			if send(w, r, dir, candidate, http.StatusOK) {
				return
			}
		}
		if !send(w, r, dir, "/404.html", http.StatusNotFound) {
			http.NotFound(w, r)
		}
	})
}

// send answers with one file of the folder, and says false when there is none.
func send(w http.ResponseWriter, r *http.Request, dir, name string, status int) bool {
	full := filepath.Join(dir, filepath.FromSlash(name))
	info, err := os.Stat(full)
	if err != nil || info.IsDir() {
		return false
	}

	header := w.Header()
	header.Set("Content-Type", contentType(name))
	switch {
	// Vite names these by their content, and a field's URL carries its
	// content id (DECISIONS 72): either is kept by a browser for good.
	case strings.HasPrefix(name, "/_app/immutable/"), strings.HasSuffix(name, ".field"):
		header.Set("Cache-Control", forGood)
	default:
		header.Set("Cache-Control", "no-cache")
	}

	// A brotli sibling made at build time travels in place of the file to a
	// browser that reads it: a field is 25 MB, and 16 as brotli (DECISIONS 72).
	if sibling := full + ".br"; strings.Contains(r.Header.Get("Accept-Encoding"), "br") {
		if packed, err := os.Stat(sibling); err == nil && !packed.IsDir() {
			header.Set("Content-Encoding", "br")
			header.Add("Vary", "Accept-Encoding")
			full, info = sibling, packed
		}
	}

	file, err := os.Open(full)
	if err != nil {
		return false
	}
	defer file.Close()
	if status != http.StatusOK {
		// ServeContent answers 200 and reads the conditions of a request; a
		// 404 page is sent as it is.
		header.Set("Cache-Control", "no-store")
		w.WriteHeader(status)
		if r.Method != http.MethodHead {
			_, _ = file.WriteTo(w)
		}
		return true
	}
	// The name is empty so the type set above stands.
	http.ServeContent(w, r, "", info.ModTime(), file)
	return true
}

// contentType is by extension, with the ones the asset set has and Go's table lacks.
func contentType(name string) string {
	switch ext := strings.ToLower(path.Ext(name)); ext {
	case ".glb", ".vrm":
		return "model/gltf-binary"
	case ".field":
		return "application/octet-stream"
	default:
		if known := mime.TypeByExtension(ext); known != "" {
			return known
		}
		return "application/octet-stream"
	}
}
