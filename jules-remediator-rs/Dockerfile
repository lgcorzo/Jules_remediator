# Base image for building
FROM rust:1.76-slim-bookworm as builder

WORKDIR /usr/src/jules-remediator
COPY . .

# Build the application
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install CA certificates for secure communication
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from:builder /usr/src/jules-remediator/target/release/jules-remediator-rs /app/jules-remediator

# Set environmental defaults
ENV JULES_DISPATCHER_URI="http://jules-cloud-vm.internal:8080/mcp"
ENV MLFLOW_TRACKING_URI="http://mlflow.ml-system.svc.cluster.local:5000"

ENTRYPOINT ["/app/jules-remediator"]
