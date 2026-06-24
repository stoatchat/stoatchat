# Build Stage
FROM ghcr.io/stoatchat/base:latest AS builder
FROM debian:bookworm-20260623 AS debian

# Bundle Stage
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /home/rust/src/target/release/revolt-january ./
COPY --from=mwader/static-ffmpeg:7.0.2 /ffmpeg /usr/local/bin/
COPY --from=mwader/static-ffmpeg:7.0.2 /ffprobe /usr/local/bin/
COPY --from=debian /usr/bin/uname /usr/bin/uname

EXPOSE 14705
USER nonroot
CMD ["./revolt-january"]
