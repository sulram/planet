package world

import (
	"errors"
	"testing"
)

func TestParseSeedRoundTrip(t *testing.T) {
	for _, text := range []string{"0000000000000000", "00000000000000ff", "ffffffffffffffff", "0123456789abcdef"} {
		seed, err := ParseSeed(text)
		if err != nil {
			t.Fatalf("ParseSeed(%q): %v", text, err)
		}
		if seed.String() != text {
			t.Errorf("ParseSeed(%q).String() = %q", text, seed.String())
		}
	}
}

func TestParseSeedRejectsOtherSpellings(t *testing.T) {
	for _, text := range []string{"", "ff", "0123456789ABCDEF", "0x23456789abcdef", "+123456789abcdef", "0123456789abcdef0", "0123456789abcdeg"} {
		if _, err := ParseSeed(text); !errors.Is(err, ErrSeed) {
			t.Errorf("ParseSeed(%q) error = %v, want ErrSeed", text, err)
		}
	}
}

func TestNewRecipe(t *testing.T) {
	const seed = "0123456789abcdef"
	cases := []struct {
		name    string
		seed    string
		version int
		params  string
		want    error
	}{
		{"valid", seed, 1, `{"sea_level": 0.4}`, nil},
		{"absent params", seed, 3, ``, nil},
		{"null params", seed, 1, `null`, nil},
		{"bad seed", "nope", 1, `{}`, ErrSeed},
		{"version zero", seed, 0, `{}`, ErrGeneratorVersion},
		{"negative version", seed, -2, `{}`, ErrGeneratorVersion},
		{"array params", seed, 1, `[1]`, ErrParams},
		{"scalar params", seed, 1, `7`, ErrParams},
		{"broken params", seed, 1, `{"a":`, ErrParams},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			_, err := NewRecipe(c.seed, c.version, []byte(c.params))
			if !errors.Is(err, c.want) {
				t.Fatalf("error = %v, want %v", err, c.want)
			}
		})
	}
}

func TestNewRecipeDefaultsParamsToEmptyObject(t *testing.T) {
	recipe, err := NewRecipe("0123456789abcdef", 1, nil)
	if err != nil {
		t.Fatal(err)
	}
	if string(recipe.Params) != "{}" {
		t.Errorf("Params = %s, want {}", recipe.Params)
	}
}

func TestRecipeEqual(t *testing.T) {
	recipe := func(seed string, version int, params string) Recipe {
		t.Helper()
		r, err := NewRecipe(seed, version, []byte(params))
		if err != nil {
			t.Fatal(err)
		}
		return r
	}
	base := recipe("0123456789abcdef", 1, `{"a":1,"b":{"c":[1,2]}}`)

	if !base.Equal(recipe("0123456789abcdef", 1, `{ "b": {"c": [1, 2]}, "a": 1 }`)) {
		t.Error("key order and whitespace must not count as a change")
	}
	if !recipe("0123456789abcdef", 1, ``).Equal(recipe("0123456789abcdef", 1, `{}`)) {
		t.Error("absent params must equal the empty object")
	}
	for name, other := range map[string]Recipe{
		"seed":    recipe("0123456789abcdee", 1, `{"a":1,"b":{"c":[1,2]}}`),
		"version": recipe("0123456789abcdef", 2, `{"a":1,"b":{"c":[1,2]}}`),
		"params":  recipe("0123456789abcdef", 1, `{"a":2,"b":{"c":[1,2]}}`),
	} {
		if base.Equal(other) {
			t.Errorf("a different %s must not be equal", name)
		}
	}
}
