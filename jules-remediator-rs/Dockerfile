FROM rust:1.88-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/jules-remediator
COPY . .

# Build the application
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install CA certificates for secure communication
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/jules-remediator/target/release/jules-remediator-rs /app/jules-remediator

# Set environmental defaults
ENV JULES_DISPATCHER_URI="http://jules-cloud-vm.internal:8080/mcp"
ENV MLFLOW_TRACKING_URI="http://mlflow.ml-system.svc.cluster.local:5000"

ENTRYPOINT ["/app/jules-remediator"]
