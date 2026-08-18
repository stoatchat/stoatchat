# Build Stage
FROM ghcr.io/stoatchat/base:latest AS builder
FROM debian:13 AS debian

# Bundle Stage
FROM gcr.io/distroless/cc-debian13:nonroot
COPY --from=builder /home/rust/src/target/release/revolt-bonfire ./
COPY --from=debian /usr/bin/uname /usr/bin/uname

EXPOSE 14703
USER nonroot
CMD ["./revolt-bonfire"]
