# Multi-stage build for prove-api with CUDA support
FROM nvidia/cuda:12.3.2-devel-ubuntu22.04 AS builder

# Support multiple GPU architectures (A100=sm_80, L40/RTX4090=sm_89, H100=sm_90)
# Override at build time with: --build-arg CUDA_ARCH_GENCODE="-gencode arch=compute_80,code=sm_80"
ARG CUDA_ARCH_GENCODE="-gencode arch=compute_80,code=sm_80 -gencode arch=compute_89,code=sm_89 -gencode arch=compute_90,code=sm_90"

ENV DEBIAN_FRONTEND=noninteractive \
    CUDA_HOME=/usr/local/cuda \
    NVCC_PREPEND_FLAGS="${CUDA_ARCH_GENCODE}" \
    RUSTUP_HOME=/root/.rustup \
    CARGO_HOME=/root/.cargo \
    RZUP_HOME=/root/.risc0 \
    LIBCLANG_PATH=/usr/lib/llvm-14/lib

# Install Rust and build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        curl \
        build-essential \
        ca-certificates \
        clang-14 \
        git \
        libclang-14-dev \
        libssl-dev \
        pkg-config \
        protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*

# Install Rust toolchain (rustup) for the host crates.
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal && \
    . "$CARGO_HOME/env" && \
    rustup toolchain install 1.90 --component rustfmt --component clippy

ENV PATH="${CARGO_HOME}/bin:${PATH}"

# Install rzup + default RISC Zero toolchains (rust, cargo-risczero, r0vm, risc0-groth16).
RUN curl -L https://risczero.com/install | bash && \
    "$RZUP_HOME/bin/rzup" install rust && \
    "$RZUP_HOME/bin/rzup" install cargo-risczero && \
    "$RZUP_HOME/bin/rzup" install r0vm && \
    "$RZUP_HOME/bin/rzup" install risc0-groth16

ENV PATH="${RZUP_HOME}/toolchains/v1.90.0-rust-x86_64-unknown-linux-gnu/bin:${RZUP_HOME}/bin:${CARGO_HOME}/bin:${CUDA_HOME}/bin:${PATH}"
ENV LD_LIBRARY_PATH="${CUDA_HOME}/lib64:${CUDA_HOME}/nvvm/lib64:${LD_LIBRARY_PATH}"

WORKDIR /build

# Copy workspace crates
COPY Cargo.toml rust-toolchain.toml .
COPY host host/
COPY methods methods/
COPY core core/
COPY email-parser email-parser/
COPY extractor extractor/
COPY smtp-receiver smtp-receiver/
COPY contract contract/
COPY proof-processor proof-processor/
COPY prove-api prove-api/

# Build the actual application with CUDA support (uses BuildKit registry cache if available)
RUN --mount=type=cache,target=/root/.cargo/registry \
    RISC0_DEV_MODE=0 RUST_LOG=info \
    cargo build --release --package prove-api --features cuda

# Runtime stage with CUDA
FROM nvidia/cuda:12.3.2-runtime-ubuntu22.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/prove-api /app/prove-api

# Copy risc0-groth16 component (required for Groth16 proof generation)
COPY --from=builder /root/.risc0 /root/.risc0
ENV RZUP_HOME=/root/.risc0
ENV PATH="${RZUP_HOME}/bin:${PATH}"

# Expose port
EXPOSE 8080

# Environment variables
ENV PORT=8080
ENV RUST_LOG=info
ENV RISC0_DEV_MODE=0

# Health check for load balancer
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["curl", "-f", "http://localhost:8080/ping"]

# Run the API server
ENTRYPOINT ["/app/prove-api"]
