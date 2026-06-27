#!/bin/sh

if [ -z "$TARGETARCH" ]; then
  :
else
  case "${TARGETARCH}" in
    "amd64")
      LINKER_NAME="x86_64-linux-gnu-gcc"
      LINKER_PACKAGE="gcc-x86-64-linux-gnu"
      BUILD_TARGET="x86_64-unknown-linux-gnu" ;;
    "arm64")
      LINKER_NAME="aarch64-linux-gnu-gcc"
      LINKER_PACKAGE="gcc-aarch64-linux-gnu"
      BUILD_TARGET="aarch64-unknown-linux-gnu" ;;
  esac
fi

tools() {
  apt-get install -y "${LINKER_PACKAGE}"
  rustup target add "${BUILD_TARGET}"
}

deps() {
  mkdir -p \
    crates/bonfire/src \
    crates/delta/src \
    crates/core/config/src \
    crates/core/database/src \
    crates/core/files/src \
    crates/core/models/src \
    crates/core/parser/src \
    crates/core/permissions/src \
    crates/core/presence/src \
    crates/core/result/src \
    crates/core/coalesced/src \
    crates/core/ratelimits/src \
    crates/services/autumn/src \
    crates/services/january/src \
    crates/services/gifbox/src \
    crates/daemons/crond/src \
    crates/daemons/pushd/src \
    crates/daemons/voice-ingress/src
  echo 'fn main() { panic!("stub"); }' |
    tee crates/bonfire/src/main.rs |
    tee crates/delta/src/main.rs |
    tee crates/services/autumn/src/main.rs |
    tee crates/services/january/src/main.rs |
    tee crates/services/gifbox/src/main.rs |
    tee crates/daemons/crond/src/main.rs |
    tee crates/daemons/pushd/src/main.rs |
    tee crates/daemons/voice-ingress/src/main.rs
  echo '' |
    tee crates/core/config/src/lib.rs |
    tee crates/core/database/src/lib.rs |
    tee crates/core/files/src/lib.rs |
    tee crates/core/models/src/lib.rs |
    tee crates/core/parser/src/lib.rs |
    tee crates/core/permissions/src/lib.rs |
    tee crates/core/presence/src/lib.rs |
    tee crates/core/result/src/lib.rs |
    tee crates/core/coalesced/src/lib.rs |
    tee crates/core/ratelimits/src/lib.rs
  
  if [ -z "$TARGETARCH" ]; then
    cargo build -j 10 --locked --release
  else
    cargo build -j 10 --locked --release --target "${BUILD_TARGET}"
  fi
}

apps() {
  touch -am \
    crates/bonfire/src/main.rs \
    crates/delta/src/main.rs \
    crates/daemons/crond/src/main.rs \
    crates/daemons/pushd/src/main.rs \
    crates/daemons/voice-ingress/src/main.rs \
    crates/core/config/src/lib.rs \
    crates/core/database/src/lib.rs \
    crates/core/models/src/lib.rs \
    crates/core/parser/src/lib.rs \
    crates/core/permissions/src/lib.rs \
    crates/core/presence/src/lib.rs \
    crates/core/result/src/lib.rs \
    crates/core/coalesced/src/lib.rs \
    crates/core/ratelimits/src/lib.rs
  
  if [ -z "$TARGETARCH" ]; then
    cargo build -j 10 --locked --release
  else
    cargo build -j 10 --locked --release --target "${BUILD_TARGET}"
    mv target _target && mv _target/"${BUILD_TARGET}" target
  fi
}

if [ -z "$TARGETARCH" ]; then
  :
else
  export RUSTFLAGS="-C linker=${LINKER_NAME}"
  export PKG_CONFIG_ALLOW_CROSS="1"
  export PKG_CONFIG_PATH="/usr/lib/pkgconfig:/usr/lib/aarch64-linux-gnu/pkgconfig:/usr/lib/x86_64-linux-gnu/pkgconfig"
fi

"$@"
