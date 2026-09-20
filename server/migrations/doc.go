// Package migrations holds the hand written schema migrations of the cold
// plane. Importing it registers them; PocketBase applies the new ones on serve.
//
// A migration is a frozen snapshot: it spells out names and rules as literals
// and never imports application code, so later refactors cannot rewrite history.
package migrations
