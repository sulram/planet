package cold

import (
	"errors"
	"fmt"

	"github.com/pocketbase/pocketbase/core"
)

// ensureInstance seeds the one instance record. Idempotent: a second start
// finds it and does nothing.
func ensureInstance(app core.App) error {
	if _, err := findInstance(app); err == nil {
		return nil
	}
	collection, err := app.FindCollectionByNameOrId(instanceCollection)
	if err != nil {
		return fmt.Errorf("ensure instance: %w", err)
	}
	if err := app.Save(core.NewRecord(collection)); err != nil {
		return fmt.Errorf("ensure instance: %w", err)
	}
	return nil
}

// findInstance is the one record, or an error when there is none yet.
func findInstance(app core.App) (*core.Record, error) {
	records, err := app.FindRecordsByFilter(instanceCollection, "", "", 1, 0)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, errors.New("no instance record")
	}
	return records[0], nil
}

// keepOneInstance refuses a second record, whoever asks: the API has no
// create rule, and this covers superusers and Go code too.
func keepOneInstance(e *core.RecordEvent) error {
	if _, err := findInstance(e.App); err == nil {
		return errors.New("the instance has exactly one record")
	}
	return e.Next()
}
