package mundos

import (
	"crypto/ed25519"
	"crypto/rand"
	"crypto/x509"
	"encoding/base64"
	"encoding/json"
	"encoding/pem"
	"errors"
	"strings"
	"testing"
	"time"
)

// A token signed by mundos's own code (`signJwt` over `claims`, in its
// repository) with a key made for this test, and that key's public half as
// mundos hands it to a world: PEM with its newlines escaped. Issued at
// 1790000000, good for a minute.
const (
	mundosKey   = `-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEAvwCxdg7QYr0xA/Gf9TDraePwoi3Lw74bf37yKzYQn+8=\n-----END PUBLIC KEY-----\n`
	mundosToken = "eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJtdW5kb3MiLCJzdWIiOiJhY2MwMDAwMDAwMDAwMDAxIiwiYXVkIjoib2ZpY2luYSIsIm5hbWUiOiJBZGEiLCJsZXZlbCI6ImJ1aWxkZXIiLCJpYXQiOjE3OTAwMDAwMDAsImV4cCI6MTc5MDAwMDA2MH0.T_Xyd6klpQQ1RTB5onTcEfIIrE4GyUSvVuWIlZQNCo1xoFQi-09jAnL9QNUcLBxXrOmuZBrPV59OBVRgbNVgBg"
)

var issued = time.Unix(1790000000, 0)

func TestATokenMundosSignedIsRead(t *testing.T) {
	key, err := ParsePublicKey(mundosKey)
	if err != nil {
		t.Fatal(err)
	}
	claims, err := Verify(mundosToken, key, "oficina", issued.Add(10*time.Second))
	if err != nil {
		t.Fatal(err)
	}
	if claims.Sub != "acc0000000000001" || claims.Name != "Ada" || claims.Level != "builder" {
		t.Fatalf("the claims are mundos's: %+v", claims)
	}

	if _, err := Verify(mundosToken, key, "atelier", issued); !errors.Is(err, ErrToken) {
		t.Fatal("a token for one world opens no other")
	}
	if _, err := Verify(mundosToken, key, "oficina", issued.Add(60*time.Second+Leeway)); err != nil {
		t.Fatalf("the leeway holds to its last second: %v", err)
	}
	if _, err := Verify(mundosToken, key, "oficina", issued.Add(61*time.Second+Leeway)); !errors.Is(err, ErrToken) {
		t.Fatal("past its minute and the leeway a token is spent")
	}
}

// sign makes a token the way mundos does, with a key of the test's own.
func sign(t *testing.T, private ed25519.PrivateKey, header, body map[string]any) string {
	t.Helper()
	part := func(v any) string {
		raw, err := json.Marshal(v)
		if err != nil {
			t.Fatal(err)
		}
		return base64.RawURLEncoding.EncodeToString(raw)
	}
	signed := part(header) + "." + part(body)
	return signed + "." + base64.RawURLEncoding.EncodeToString(ed25519.Sign(private, []byte(signed)))
}

func TestWhatIsRefused(t *testing.T) {
	public, private, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	_, other, _ := ed25519.GenerateKey(rand.Reader)
	now := time.Unix(1790000000, 0)
	eddsa := map[string]any{"alg": "EdDSA", "typ": "JWT"}
	good := func() map[string]any {
		return map[string]any{"iss": "mundos", "sub": "a1", "aud": "oficina", "name": "Ada", "level": "admin", "exp": now.Unix() + 60}
	}
	with := func(key string, value any) map[string]any {
		body := good()
		if value == nil {
			delete(body, key)
		} else {
			body[key] = value
		}
		return body
	}

	if _, err := Verify(sign(t, private, eddsa, good()), public, "oficina", now); err != nil {
		t.Fatalf("the good one passes: %v", err)
	}
	forged := sign(t, private, eddsa, good())
	admin := sign(t, private, eddsa, with("level", "builder"))
	// The body of one token under the signature of another.
	swapped := strings.Split(admin, ".")[0] + "." + strings.Split(forged, ".")[1] + "." + strings.Split(admin, ".")[2]

	for name, token := range map[string]string{
		"signed by another key":  sign(t, other, eddsa, good()),
		"a body it did not sign": swapped,
		"no signature algorithm": sign(t, private, map[string]any{"alg": "none"}, good()),
		"another issuer":         sign(t, private, eddsa, with("iss", "someone")),
		"no account":             sign(t, private, eddsa, with("sub", nil)),
		"no expiry":              sign(t, private, eddsa, with("exp", nil)),
		"not a token":            "guest",
		"empty":                  "",
	} {
		if _, err := Verify(token, public, "oficina", now); !errors.Is(err, ErrToken) {
			t.Errorf("%s is refused: %v", name, err)
		}
	}
}

func TestTheKeyIsReadAsMundosWritesIt(t *testing.T) {
	public, _, _ := ed25519.GenerateKey(rand.Reader)
	der, _ := x509.MarshalPKIXPublicKey(public)
	text := string(pem.EncodeToMemory(&pem.Block{Type: "PUBLIC KEY", Bytes: der}))

	for name, written := range map[string]string{"with newlines": text, "with them escaped": strings.ReplaceAll(text, "\n", `\n`)} {
		read, err := ParsePublicKey(written)
		if err != nil || !read.Equal(public) {
			t.Errorf("a key %s is read: %v", name, err)
		}
	}
	if _, err := ParsePublicKey("not a key"); err == nil {
		t.Error("what is not PEM is refused")
	}
}
