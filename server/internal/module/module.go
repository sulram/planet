// Package module runs the server's module (DECISIONS 97, 99, 102): every
// world half of a version, written in Rust, built as one WASM file and run
// here through wazero. It is the one package that knows the module is WASM.
// The core meets each plugin it carries as a world.Plugin, and the bridge
// between the two is the schema of proto/planet/module/v1.
package module

import (
	"bytes"
	"context"
	_ "embed"
	"errors"
	"fmt"
	"log"
	"slices"
	"sync"
	"time"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
	"google.golang.org/protobuf/proto"

	pb "github.com/sulram/planet/server/internal/protocol"
	bridge "github.com/sulram/planet/server/internal/protocol/module"
	"github.com/sulram/planet/server/internal/world"
)

// The module, built from crates/module by `bun run module`.
//
//go:embed world.wasm
var built []byte

const (
	// A call ends within this long, or the instance is replaced: a world
	// half that never returns stops no world.
	deadline = 250 * time.Millisecond
	// The most a module holds, in pages of 64 KiB: 64 MB.
	memoryPages = 1024
	// The most a call is answered with: this many replies, each this long.
	// The ceiling of memory is the module's; these are the server's, for what
	// it keeps of what the module says.
	maxReplies   = 1024
	maxReplySize = 1 << 20
)

// Module is the world halves of a version, running. One call at a time.
type Module struct {
	mu       sync.Mutex
	runtime  wazero.Runtime
	compiled wazero.CompiledModule
	// The instance that serves calls. Replaced after any fault.
	instance api.Module
	// How many instances there have been.
	born int
	// The recipe the instance was started with. Nil until a world starts,
	// and again once the instance is replaced.
	started *world.Recipe
	// The call in flight: what the module said so far, and what went wrong
	// hearing it.
	replies []*bridge.Reply
	fault   error
	// What the module carries, as it said at Load.
	carried []*bridge.Carried
}

// Load compiles the module built into this binary and asks what it carries.
func Load(ctx context.Context) (*Module, error) {
	m, err := open(ctx, built)
	if err != nil {
		return nil, err
	}
	replies, err := m.call(ctx, &bridge.Call{Call: &bridge.Call_Describe{Describe: &bridge.Describe{}}})
	if err != nil {
		return nil, fmt.Errorf("module: asked what it carries: %w", err)
	}
	for _, reply := range replies {
		if statement := reply.GetStatement(); statement != nil {
			m.carried = statement.Plugins
			return m, nil
		}
	}
	return nil, errors.New("module: it said nothing of what it carries")
}

// open compiles a module and starts its first instance.
func open(ctx context.Context, wasm []byte) (*Module, error) {
	m := &Module{}
	// A call ends when its context does, and a module's memory has a ceiling.
	m.runtime = wazero.NewRuntimeWithConfig(ctx, wazero.NewRuntimeConfig().
		WithCloseOnContextDone(true).
		WithMemoryLimitPages(memoryPages))
	if _, err := m.runtime.NewHostModuleBuilder("host").
		NewFunctionBuilder().WithFunc(m.reply).Export("reply").
		Instantiate(ctx); err != nil {
		return nil, fmt.Errorf("module: %w", err)
	}
	compiled, err := m.runtime.CompileModule(ctx, wasm)
	if err != nil {
		return nil, fmt.Errorf("module: %w", err)
	}
	// The two names the server calls, as it calls them, and a memory to
	// write a call in: a file without them is no module.
	exports := compiled.ExportedFunctions()
	i32 := []api.ValueType{api.ValueTypeI32}
	for name, want := range map[string][2][]api.ValueType{"reserve": {i32, i32}, "call": {nil, nil}} {
		got, there := exports[name]
		if !there || !slices.Equal(got.ParamTypes(), want[0]) || !slices.Equal(got.ResultTypes(), want[1]) {
			return nil, fmt.Errorf("module: it does not export `%s` as the bridge calls it", name)
		}
	}
	if len(compiled.ExportedMemories()) == 0 {
		return nil, errors.New("module: it exports no memory")
	}
	m.compiled = compiled
	if err := m.replace(ctx); err != nil {
		return nil, err
	}
	return m, nil
}

// Close lets the runtime go.
func (m *Module) Close(ctx context.Context) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.runtime.Close(ctx)
}

// replace starts a new instance in place of the one there is. What a world
// half held in memory goes with the old one, and the world starts again in
// the new one before its next op.
func (m *Module) replace(ctx context.Context) error {
	if m.instance != nil {
		_ = m.instance.Close(ctx)
	}
	m.instance, m.started = nil, nil
	m.born++
	instance, err := m.runtime.InstantiateModule(ctx, m.compiled,
		wazero.NewModuleConfig().WithName(fmt.Sprintf("world-%d", m.born)))
	if err != nil {
		return fmt.Errorf("module: %w", err)
	}
	m.instance = instance
	return nil
}

// reply is what the module calls to hand over one reply, whole. The bytes
// are read within the module's memory or refused, and copied before the
// module runs again. The first reply that cannot be taken ends the call: the
// instance is closed where it stands, so a module that goes on saying is not
// heard until its deadline.
func (m *Module) reply(ctx context.Context, instance api.Module, at, size uint32) {
	if m.fault != nil {
		return
	}
	if m.fault = m.take(instance, at, size); m.fault != nil {
		_ = instance.CloseWithExitCode(ctx, 1)
	}
}

func (m *Module) take(instance api.Module, at, size uint32) error {
	if len(m.replies) == maxReplies {
		return fmt.Errorf("more than %d replies to one call", maxReplies)
	}
	if size > maxReplySize {
		return fmt.Errorf("a reply of %d bytes", size)
	}
	view, ok := instance.Memory().Read(at, size)
	if !ok {
		return errors.New("a reply outside the module's memory")
	}
	reply := &bridge.Reply{}
	if err := proto.Unmarshal(bytes.Clone(view), reply); err != nil {
		return fmt.Errorf("a reply that is no message: %w", err)
	}
	m.replies = append(m.replies, reply)
	return nil
}

// call asks the module one thing and gathers what it says back. After any
// fault, a trap, a deadline or a reply that cannot be read, the instance is
// replaced and the call is lost.
func (m *Module) call(ctx context.Context, call *bridge.Call) ([]*bridge.Reply, error) {
	if m.instance == nil {
		if err := m.replace(ctx); err != nil {
			return nil, err
		}
	}
	frame, err := proto.Marshal(call)
	if err != nil {
		return nil, err
	}
	m.replies, m.fault = nil, nil
	if err := m.ask(ctx, frame); err != nil {
		if again := m.replace(ctx); again != nil {
			log.Printf("module: no new instance: %v", again)
		}
		return nil, err
	}
	return m.replies, nil
}

// ask writes a call into the inbox the module sized for it, and runs it.
func (m *Module) ask(ctx context.Context, frame []byte) error {
	ctx, cancel := context.WithTimeout(ctx, deadline)
	defer cancel()
	at, err := m.instance.ExportedFunction("reserve").Call(ctx, uint64(len(frame)))
	if err != nil {
		return err
	}
	if !m.instance.Memory().Write(uint32(at[0]), frame) {
		return errors.New("an inbox outside the module's memory")
	}
	_, err = m.instance.ExportedFunction("call").Call(ctx)
	// What went wrong hearing a reply says more than the exit it caused.
	if m.fault != nil {
		return m.fault
	}
	return err
}

// start tells the instance which world this is, once for each instance and
// each recipe.
func (m *Module) start(ctx context.Context, recipe world.Recipe) error {
	if s := m.started; s != nil && s.Seed == recipe.Seed && s.GeneratorVersion == recipe.GeneratorVersion && bytes.Equal(s.Params, recipe.Params) {
		return nil
	}
	if _, err := m.call(ctx, &bridge.Call{Call: &bridge.Call_Start{Start: &bridge.Start{Recipe: recipe.Wire()}}}); err != nil {
		return err
	}
	m.started = &recipe
	return nil
}

// Plugins is the plugins the module carries, each as the core hosts it, in
// the order the config lists them.
func (m *Module) Plugins() []world.Installed {
	installed := make([]world.Installed, 0, len(m.carried))
	for _, carried := range m.carried {
		ops := make([]world.Op, 0, len(carried.Ops))
		for _, op := range carried.Ops {
			ops = append(ops, world.Op{Kind: op.Kind, Level: world.Level(op.Level)})
		}
		installed = append(installed, world.Installed{
			Plugin: &hosted{module: m, name: carried.Name, version: carried.Version, ops: ops},
			On:     carried.On,
		})
	}
	return installed
}

// hosted is one plugin's world half, as the core's host of plugins sees it.
type hosted struct {
	module  *Module
	name    string
	version uint32
	ops     []world.Op
}

func (h *hosted) Name() string    { return h.name }
func (h *hosted) Version() uint32 { return h.version }
func (h *hosted) Ops() []world.Op { return h.ops }

// Do hands the op to the world half with the room it is asked in, and tells
// the room what the half said. An op the module faults on is dropped, as an
// op that stops at the host is.
func (h *hosted) Do(room world.Room, who world.Who, kind string, payload []byte) {
	m := h.module
	m.mu.Lock()
	defer m.mu.Unlock()
	ctx := context.Background()
	if err := m.start(ctx, room.Recipe()); err != nil {
		log.Printf("module: the world did not start: %v", err)
		return
	}
	here := room.Here()
	op := &bridge.Op{
		Plugin:  h.name,
		Kind:    kind,
		Payload: payload,
		Session: who.Session,
		NowMs:   uint64(room.Now().UnixMilli()),
		Here:    make([]*bridge.Who, 0, len(here)),
	}
	for _, other := range here {
		op.Here = append(op.Here, &bridge.Who{
			Session: other.Session,
			User:    other.Identity.UserID,
			Name:    other.Name,
			Level:   pb.Level(other.Identity.Level),
			Stance:  other.Stance,
		})
	}
	replies, err := m.call(ctx, &bridge.Call{Call: &bridge.Call_Op{Op: op}})
	if err != nil {
		log.Printf("module: %s.%s is dropped: %v", h.name, kind, err)
		return
	}
	for _, reply := range replies {
		tell := reply.GetTell()
		// A world half speaks under its own name alone.
		if tell == nil || tell.Plugin != h.name {
			continue
		}
		to := make(map[uint32]bool, len(tell.To))
		for _, session := range tell.To {
			to[session] = true
		}
		room.Tell(tell.Kind, tell.Payload, func(other world.Who) bool { return to[other.Session] })
	}
}

// Gone tells the world half a session left.
func (h *hosted) Gone(session uint32) {
	m := h.module
	m.mu.Lock()
	defer m.mu.Unlock()
	gone := &bridge.Gone{Plugin: h.name, Session: session}
	if _, err := m.call(context.Background(), &bridge.Call{Call: &bridge.Call_Gone{Gone: gone}}); err != nil {
		log.Printf("module: %s is not told who left: %v", h.name, err)
	}
}
