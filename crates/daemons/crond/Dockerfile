# Build Stage
ARG BASE_IMAGE=ghcr.io/stoatchat/base:latest
FROM $BASE_IMAGE AS builder
FROM debian:12 AS debian

# Bundle Stage
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /home/rust/src/target/release/revolt-crond ./
COPY --from=debian /usr/bin/uname /usr/bin/uname

USER nonroot
CMD ["./revolt-crond"]