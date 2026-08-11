# Build Stage
ARG BASE_IMAGE=ghcr.io/stoatchat/base:latest
FROM $BASE_IMAGE AS builder
FROM debian:12 AS debian

# Bundle Stage
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /home/rust/src/target/release/revolt-gifbox ./

EXPOSE 14706
USER nonroot
CMD ["./revolt-gifbox"]
