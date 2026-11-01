# syntax=docker/dockerfile:1

FROM rust:1.99-slim-trixie AS build
WORKDIR /app

# Compile dependencies against a placeholder main so this layer stays cached
# until Cargo.toml / Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
 && cargo build --release --locked

COPY src ./src
# touch: make cargo rebuild the real main.rs instead of reusing the placeholder.
RUN touch src/main.rs && cargo build --release --locked

# glibc runtime matching the trixie (Debian 13) builder; runs as uid 65532.
FROM gcr.io/distroless/cc-debian13:nonroot
COPY --from=build /app/target/release/echo-api /usr/local/bin/echo-api
ENV PORT=8080
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/echo-api"]
