package cold

import (
	"fmt"
	"os"
	"strconv"
	"strings"
)

// Config is what the environment decides about the cold plane. The
// environment wins over the admin panel: it is applied on every start.
type Config struct {
	// AppURL is the public URL of the web app, the base of every magic link.
	AppURL        string
	AppName       string
	SenderAddress string
	SMTP          SMTP
	// OperatorEmail names the account that is granted the operator flag on
	// start. Empty grants nothing.
	OperatorEmail string
}

// SMTP with an empty Host means mail is off and sign in links go to the log.
type SMTP struct {
	Host     string
	Port     int
	Username string
	Password string
}

// ConfigFromEnv reads the process environment and fills the defaults of a
// local development setup.
func ConfigFromEnv() (Config, error) {
	port, err := strconv.Atoi(envOr("SMTP_PORT", "587"))
	if err != nil {
		return Config{}, fmt.Errorf("SMTP_PORT: %w", err)
	}
	return Config{
		AppURL:  strings.TrimRight(envOr("APP_URL", "http://localhost:5173"), "/"),
		AppName: envOr("APP_NAME", "planet"),
		// PocketBase rejects a sender without a dot in the domain, so the
		// default uses the reserved .localhost name.
		SenderAddress: envOr("SMTP_SENDER", "planet@planet.localhost"),
		SMTP: SMTP{
			Host:     os.Getenv("SMTP_HOST"),
			Port:     port,
			Username: os.Getenv("SMTP_USERNAME"),
			Password: os.Getenv("SMTP_PASSWORD"),
		},
		OperatorEmail: os.Getenv("PLANET_OPERATOR_EMAIL"),
	}, nil
}

// envOr treats an empty variable as unset, which is how a blank line in .env
// reads.
func envOr(key, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}
