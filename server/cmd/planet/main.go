// Command planet is the one server binary: PocketBase as the cold plane, with
// the world server registered inside it as it grows.
package main

import (
	"log"
	"os"
	"path/filepath"

	"github.com/pocketbase/pocketbase"
	"github.com/pocketbase/pocketbase/plugins/migratecmd"
	"github.com/pocketbase/pocketbase/tools/osutils"

	"github.com/sulram/planet/server/internal/cold"
)

func main() {
	cfg, err := cold.ConfigFromEnv()
	if err != nil {
		log.Fatal(err)
	}

	app := pocketbase.NewWithConfig(pocketbase.Config{
		DefaultDataDir: defaultDataDir(),
	})
	cold.Register(app, cfg)

	// The migrate command only: migrations are hand written, so the schema
	// never changes behind the back of the repo.
	migratecmd.MustRegister(app, app.RootCmd, migratecmd.Config{
		TemplateLang: migratecmd.TemplateLangGo,
		Automigrate:  false,
	})

	if err := app.Start(); err != nil {
		log.Fatal(err)
	}
}

// defaultDataDir is server/pb_data wherever the process starts from. The
// binary is built to server/bin, so the data sits beside that folder; under
// `go run` the executable is a temp file and the working directory is server/.
// The --dir flag still overrides it.
func defaultDataDir() string {
	if osutils.IsProbablyGoRun() {
		return "pb_data"
	}
	executable, err := os.Executable()
	if err != nil {
		return "pb_data"
	}
	return filepath.Join(filepath.Dir(executable), "..", "pb_data")
}
