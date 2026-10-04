# One image is one version of planet (docs/DEPLOY.md): the Go binary with the
# module of world halves in it, the page as files, the engine's WASM and the
# default asset set. The three builds run on the machine that builds, whatever
# the image is for: the engine, the module and the page are the same on every
# architecture, and Go cross compiles.
#
#   docker build -t ghcr.io/sulram/planet:<version> --build-arg VERSION=<version> .

# The engine: shell-web compiled to WASM, with the bindings the page loads.
# And the module: the plugins' world halves as one WASM file the server embeds.
FROM --platform=$BUILDPLATFORM rust:1 AS engine
WORKDIR /src
RUN rustup target add wasm32-unknown-unknown
# wasm-bindgen-cli matches the `wasm-bindgen` crate in Cargo.lock, as
# `bun run setup` holds it on a development machine.
COPY Cargo.lock ./
RUN cargo install wasm-bindgen-cli --locked --version \
	"$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')"
COPY Cargo.toml ./
COPY crates crates
# A plugin's crate is its folder (DECISIONS 100).
COPY plugins plugins
RUN --mount=type=cache,target=/usr/local/cargo/registry \
	--mount=type=cache,target=/src/target \
	cargo build --release --target wasm32-unknown-unknown -p shell-web -p module \
	&& wasm-bindgen --target web --out-dir /pkg target/wasm32-unknown-unknown/release/shell_web.wasm \
	&& cp target/wasm32-unknown-unknown/release/module.wasm /world.wasm

# The page: SvelteKit built into plain files, the asset set beside them.
FROM --platform=$BUILDPLATFORM oven/bun:1 AS web
WORKDIR /src
COPY package.json bun.lock ./
COPY apps/web/package.json apps/web/
# A plugin's panel is a package of the workspace, in the plugin's folder.
COPY plugins plugins
RUN bun install --frozen-lockfile
COPY apps/web apps/web
# `apps/web/static/assets` is a link to this folder: the build copies the set.
COPY assets assets
COPY --from=engine /pkg apps/web/src/lib/engine/pkg
RUN bun run --cwd apps/web build

# The server: one static binary for the architecture the image is for.
FROM --platform=$BUILDPLATFORM golang:1.26 AS server
ARG TARGETOS=linux
ARG TARGETARCH
ARG VERSION=dev
WORKDIR /src/server
COPY server/go.mod server/go.sum ./
RUN go mod download
COPY server ./
COPY --from=engine /world.wasm internal/module/world.wasm
RUN CGO_ENABLED=0 GOOS=$TARGETOS GOARCH=$TARGETARCH \
	go build -trimpath -ldflags "-s -w -X main.version=$VERSION" -o /planet ./cmd/planet

FROM alpine:3
# The world folder belongs to the one user the server runs as. A volume
# mounted here for the first time takes this ownership, so mundos mounts a
# generation's folder at /world, and the copy into a next generation too.
RUN adduser -D -u 1001 planet && mkdir /world && chown planet:planet /world
COPY --from=server /planet /usr/local/bin/planet
COPY --from=web /src/apps/web/build /web
ENV HOST=0.0.0.0 \
	PORT=3000 \
	WORLD_DIR=/world \
	WEB_DIR=/web
USER planet
VOLUME /world
EXPOSE 3000
# mundos reads this to say a generation is running.
HEALTHCHECK --interval=5s --timeout=3s --start-period=5s --retries=5 \
	CMD wget -qO /dev/null "http://127.0.0.1:${PORT}/api/health" || exit 1
ENTRYPOINT ["planet"]
CMD ["serve"]
