# ---- build stage ----
FROM rust:1.96-slim AS build
RUN apt-get update && apt-get install -y --no-install-recommends \
      pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
# Frontend assets and SQL migrations are embedded into the binary at compile time.
RUN cargo build --release --locked

# ---- runtime stage ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/followup /usr/local/bin/followup

# Defaults; override the rest (RP_ID, RP_ORIGIN, EXPORT_TOKEN, SESSION_SECURE=true) at runtime.
ENV BIND_ADDR=0.0.0.0:8080 \
    DATABASE_URL=sqlite:///data/app.db?mode=rwc
EXPOSE 8080
VOLUME ["/data"]
ENTRYPOINT ["/usr/local/bin/followup"]
