# Build Stage
FROM ghcr.io/stoatchat/base:latest AS builder
FROM debian:bookworm-20260623 AS debian

# Bundle Stage
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /home/rust/src/target/release/revolt-delta ./
COPY --from=debian /usr/bin/uname /usr/bin/uname

EXPOSE 14702
ENV ROCKET_ADDRESS=0.0.0.0
USER nonroot
CMD ["./revolt-delta"]
