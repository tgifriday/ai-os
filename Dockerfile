FROM rust:1.94-bookworm AS builder

WORKDIR /aios
COPY Cargo.toml Cargo.toml
COPY aios-kernel/ aios-kernel/
COPY aios-core/ aios-core/
COPY aios-shell/ aios-shell/
COPY aios-llm/ aios-llm/
COPY aios-init/ aios-init/
COPY aios-knowledge/ aios-knowledge/

RUN cargo build --workspace --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /aios/target/release/aish /usr/local/bin/aish
COPY --from=builder /aios/target/release/aios-os /usr/local/bin/aios-os
COPY --from=builder /aios/target/release/aios-init /usr/local/bin/aios-init
COPY config/ /etc/aios/

RUN mkdir -p /var/aios/models

ENV TERM=xterm-256color
ENV RUST_LOG=info

ENTRYPOINT ["/usr/local/bin/aish"]
