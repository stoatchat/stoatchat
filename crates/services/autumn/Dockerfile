# Build Stage
FROM ghcr.io/stoatchat/base:latest AS builder
FROM debian:13 AS debian
RUN apt-get update && \
    apt-get install -y --no-install-recommends libdav1d7 && \
    mkdir -p /runtime-libs && \
    for library in /usr/lib/*-linux-gnu/libdav1d.so.7*; do \
        cp -a --parents "$library" /runtime-libs; \
    done

# Bundle Stage
FROM gcr.io/distroless/cc-debian13:nonroot
COPY --from=builder /home/rust/src/target/release/revolt-autumn ./
COPY --from=debian /runtime-libs/ /
COPY --from=mwader/static-ffmpeg:7.1.1 /ffmpeg /usr/local/bin/
COPY --from=mwader/static-ffmpeg:7.1.1 /ffprobe /usr/local/bin/
COPY --from=debian /usr/bin/uname /usr/bin/uname

EXPOSE 14704
USER nonroot
CMD ["./revolt-autumn"]
