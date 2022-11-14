# =============================================================================
# Core: Build
# =============================================================================

ARG RUST_NIGHTLY_VERSION="2022-11-09"

# Application directory
ARG APP_HOME="/var/app"

FROM lukemathwalker/cargo-chef:latest-rust-slim-bullseye AS chef

WORKDIR /tmp/build

# Plan
# -----------------------------------------------------------------------------
FROM chef AS plan

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

# Build
# -----------------------------------------------------------------------------
FROM chef AS build

# Install build dependencies
RUN apt-get update && apt-get install --no-install-recommends -y \
    libssl-dev \
    pkg-config \
    && apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false \
    && rm -rf /var/lib/apt/lists/*

COPY --from=plan /tmp/build/recipe.json ./

RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

RUN cargo build --release

# =============================================================================
# Core: Base
# =============================================================================
FROM debian:bullseye-slim AS base

ARG APP_HOME

# App user (worker) for manual UID and GID set
ARG UID="1000"
ARG GID="1000"

SHELL ["/bin/bash", "-c"]

# Install runtime dependencies
RUN apt-get update && apt-get install --no-install-recommends -y \
    curl \
    libssl-dev \
    && apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false \
    && rm -rf /var/lib/apt/lists/*

# Change working directory
WORKDIR "${APP_HOME}"

# Create app user and set as app owner
RUN groupadd --gid "${GID}" worker \
    && useradd  --system --uid "${UID}" --gid "${GID}" --create-home worker \
    && chown -R worker:worker "${APP_HOME}" /home/worker

EXPOSE 1080 8080

HEALTHCHECK --interval=15s --timeout=2s --start-period=3s --retries=5 \
    CMD ["curl", "-fsSL", "localhost:8080/ht"]

ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["start.sh"]

# =============================================================================
# Environment: Development
# =============================================================================
FROM base AS development

ARG RUST_NIGHTLY_VERSION
ARG APP_HOME

ARG GRCOV_VERSION="v0.8.13"
ARG CARGO_WATCH_VERSION="v8.1.2"

# Original base directories for `rustup`, `cargo` from build stage
ENV RUSTUP_HOME="/usr/local/rustup"
ENV CARGO_HOME="/usr/local/cargo"

# Update $PATH for Rust
ENV PATH="${CARGO_HOME}/bin:${PATH}"

VOLUME ["${APP_HOME}/target"]

# Install dev dependencies & utils
RUN apt-get update && apt-get install --no-install-recommends -y \
    build-essential \
    default-jre \
    git \
    gnupg2 \
    jq \
    make \
    pkg-config \
    python3-pip \
    && apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false \
    && rm -rf /var/lib/apt/lists/*

# Install pre-commit
RUN pip3 install --no-cache-dir pre-commit

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
    -y \
    --default-toolchain "nightly-${RUST_NIGHTLY_VERSION}" \
    --profile minimal \
    --component rustfmt \
    --component clippy \
    --component llvm-tools-preview \
    && chown -R worker:worker "${RUSTUP_HOME}" "${CARGO_HOME}"

# Download grcov binary
RUN curl -fsSL "https://github.com/mozilla/grcov/releases/download/${GRCOV_VERSION}/grcov-$(rustc -vV | sed -n 's|host: ||p').tar.bz2" \
    | tar --extract --bzip2 --directory "${CARGO_HOME}/bin"

# Download cargo-watch binary
RUN mkdir --parents /tmp/cargo-watch \
    && curl -fsSL "https://github.com/watchexec/cargo-watch/releases/download/${CARGO_WATCH_VERSION}/cargo-watch-${CARGO_WATCH_VERSION}-$(rustc -vV | sed -n 's|host: ||p').tar.xz" \
    | tar --extract --xz --directory /tmp/cargo-watch --strip-components 1 \
    && mv /tmp/cargo-watch/cargo-watch "${CARGO_HOME}/bin" \
    && rm -rf /tmp/cargo-watch

# NOTE: Do not copy from build at base stage as it invalidates layers in development
#       when new build created, increasing overall build time.
COPY --from=build --chown=worker:worker --chmod=755 /tmp/build/target/release/cli /usr/local/bin/app

# Copy script files to executable path
COPY --chown=worker:worker --chmod=755 ./scripts/* /usr/local/bin/

# Create and grant permission for target directory as there is no original to preserve permission from.
RUN mkdir "${APP_HOME}/target" && chown worker:worker "${APP_HOME}/target"

USER worker:worker

# =============================================================================
# Environment: Production
# =============================================================================
FROM base AS production

COPY --from=build --chown=worker:worker --chmod=755 /tmp/build/target/release/cli /usr/local/bin/app

# Copy script files to executable path
COPY --chown=worker:worker --chmod=755 ./scripts/* /usr/local/bin/

USER worker:worker
