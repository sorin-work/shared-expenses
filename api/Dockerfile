FROM rust:1-slim-trixie AS build
WORKDIR /src

# Build the dependencies first so they are cached between source changes.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
  && echo 'fn main() {}' > src/main.rs \
  && touch src/lib.rs \
  && cargo build --release --locked \
  && rm -rf src

COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

FROM debian:trixie-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --uid 10001 --no-create-home app
COPY --from=build /src/target/release/shared-expenses-api /usr/local/bin/api
USER app
ENV PORT=4407
EXPOSE 4407
ENTRYPOINT ["/usr/local/bin/api"]
