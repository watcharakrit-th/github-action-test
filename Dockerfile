# syntax=docker/dockerfile:1

# Alpine's Rust toolchain targets musl, which links fully statically, so the
# binary needs no libc at runtime and the final image can start from scratch.
FROM rust:1.99-alpine AS build
WORKDIR /app

# Compile dependencies against a placeholder main so this layer stays cached
# until Cargo.toml / Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
 && cargo build --release --locked

COPY src ./src
# Compiled into GET /version; CI passes the commit SHA.
ARG GIT_SHA=dev
# touch: make cargo rebuild the real main.rs instead of reusing the placeholder.
RUN touch src/main.rs && cargo build --release --locked

# Nothing but the binary: no shell, no libc, no package manager.
FROM scratch
COPY --from=build /app/target/release/echo-api /echo-api
ENV PORT=8080
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["/echo-api"]
