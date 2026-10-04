// Package mundos checks what mundos signs. mundos says who each person is
// for one entry into one world: a JWT signed with Ed25519, which a world
// reads with the public key it was given and can never forge.
package mundos

import (
	"crypto/ed25519"
	"crypto/x509"
	"encoding/base64"
	"encoding/json"
	"encoding/pem"
	"errors"
	"strings"
	"time"
)

// Leeway is the clock drift allowed on a token's expiry.
const Leeway = 30 * time.Second

// ErrToken is every reason a token is refused. Why is not said: whoever
// holds a bad token is a visitor either way.
var ErrToken = errors.New("the token is not valid")

// Claims is what a token says: the account's id, its name, and its level in
// this world as mundos writes it (`admin`, `builder`, `signed_in`,
// `anonymous`).
type Claims struct {
	Sub   string `json:"sub"`
	Name  string `json:"name"`
	Level string `json:"level"`
}

// ParsePublicKey reads the key as mundos hands it over: PEM (SPKI), with its
// newlines escaped so the value fits one line of an env file.
func ParsePublicKey(text string) (ed25519.PublicKey, error) {
	block, _ := pem.Decode([]byte(strings.ReplaceAll(text, `\n`, "\n")))
	if block == nil {
		return nil, errors.New("the public key is not PEM")
	}
	parsed, err := x509.ParsePKIXPublicKey(block.Bytes)
	if err != nil {
		return nil, err
	}
	key, ok := parsed.(ed25519.PublicKey)
	if !ok {
		return nil, errors.New("the public key is not Ed25519")
	}
	return key, nil
}

// Verify reads a token mundos signed for `world`. The signature is checked
// before a claim is believed.
func Verify(token string, key ed25519.PublicKey, world string, now time.Time) (Claims, error) {
	parts := strings.Split(token, ".")
	if len(parts) != 3 {
		return Claims{}, ErrToken
	}
	var header struct {
		Alg string `json:"alg"`
	}
	if !decode(parts[0], &header) || header.Alg != "EdDSA" {
		return Claims{}, ErrToken
	}
	signature, err := base64.RawURLEncoding.DecodeString(parts[2])
	if err != nil || !ed25519.Verify(key, []byte(parts[0]+"."+parts[1]), signature) {
		return Claims{}, ErrToken
	}

	var body struct {
		Claims
		Iss string `json:"iss"`
		// The world's name, never an address: a token for one world opens no other.
		Aud string `json:"aud"`
		Exp int64  `json:"exp"`
	}
	if !decode(parts[1], &body) {
		return Claims{}, ErrToken
	}
	if body.Iss != "mundos" || body.Aud != world || body.Sub == "" {
		return Claims{}, ErrToken
	}
	if body.Exp == 0 || now.After(time.Unix(body.Exp, 0).Add(Leeway)) {
		return Claims{}, ErrToken
	}
	return body.Claims, nil
}

func decode(part string, into any) bool {
	raw, err := base64.RawURLEncoding.DecodeString(part)
	return err == nil && json.Unmarshal(raw, into) == nil
}
