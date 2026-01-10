FROM rust:1-slim-trixie AS build
WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --locked

FROM debian:trixie-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/shared-expenses-api /usr/local/bin/api
ENV PORT=4407
EXPOSE 4407
ENTRYPOINT ["/usr/local/bin/api"]
