package world

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"reflect"
	"strconv"
)

// Validation failures, one per recipe part, so a caller can map each to its
// own input field.
var (
	ErrSeed             = errors.New("seed must be 16 lowercase hex digits")
	ErrGeneratorVersion = errors.New("generator version must be 1 or greater")
	ErrParams           = errors.New("params must be a JSON object")
)

// seedDigits is the width of a u64 written in hex.
const seedDigits = 16

// Seed is the u64 that feeds the generator. Its text form is fixed width
// lowercase hex, because JSON numbers and SQLite integers cannot hold a full
// u64 and a single canonical spelling keeps recipes comparable as text.
type Seed uint64

// ParseSeed reads the canonical text form and rejects every other spelling.
func ParseSeed(text string) (Seed, error) {
	if len(text) != seedDigits {
		return 0, ErrSeed
	}
	for _, c := range []byte(text) {
		if !(c >= '0' && c <= '9' || c >= 'a' && c <= 'f') {
			return 0, ErrSeed
		}
	}
	value, err := strconv.ParseUint(text, 16, 64)
	if err != nil {
		return 0, ErrSeed
	}
	return Seed(value), nil
}

func (s Seed) String() string {
	return fmt.Sprintf("%0*x", seedDigits, uint64(s))
}

// Recipe is everything needed to regenerate the untouched terrain of a world.
// It is frozen at creation: changing any part would silently rewrite terrain
// under chunks that were stored against the old one.
type Recipe struct {
	Seed             Seed
	GeneratorVersion int
	// Params is a JSON object, never null. The generator version owns its
	// schema, so the core only guarantees the shape.
	Params json.RawMessage
}

// NewRecipe validates the parts and returns the recipe. Absent params mean
// "all defaults" and become the empty object.
func NewRecipe(seed string, generatorVersion int, params []byte) (Recipe, error) {
	parsed, err := ParseSeed(seed)
	if err != nil {
		return Recipe{}, err
	}
	if generatorVersion < 1 {
		return Recipe{}, ErrGeneratorVersion
	}
	object, err := paramsObject(params)
	if err != nil {
		return Recipe{}, err
	}
	return Recipe{Seed: parsed, GeneratorVersion: generatorVersion, Params: object}, nil
}

// Equal compares params by meaning, so key order and whitespace never count
// as a change to the recipe.
func (r Recipe) Equal(other Recipe) bool {
	if r.Seed != other.Seed || r.GeneratorVersion != other.GeneratorVersion {
		return false
	}
	var a, b map[string]any
	if json.Unmarshal(r.Params, &a) != nil || json.Unmarshal(other.Params, &b) != nil {
		return false
	}
	return reflect.DeepEqual(a, b)
}

func paramsObject(params []byte) (json.RawMessage, error) {
	trimmed := bytes.TrimSpace(params)
	if len(trimmed) == 0 || bytes.Equal(trimmed, []byte("null")) {
		return json.RawMessage("{}"), nil
	}
	var object map[string]json.RawMessage
	if err := json.Unmarshal(trimmed, &object); err != nil || object == nil {
		return nil, ErrParams
	}
	var compact bytes.Buffer
	if err := json.Compact(&compact, trimmed); err != nil {
		return nil, ErrParams
	}
	return compact.Bytes(), nil
}
