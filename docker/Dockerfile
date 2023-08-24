FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge
SHELL ["/bin/bash", "-c"]

# Install dependencies
ENV DEBIAN_FRONTEND noninteractive

RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt install --no-install-recommends fontconfig:arm64 libavcodec-dev:arm64 \ 
    libavdevice-dev:arm64 libavfilter-dev:arm64 \
    libavformat-dev:arm64 libavutil-dev:arm64 pkg-config:arm64 \
    libswscale-dev:arm64 -y

# Do links for cross-compiling weirdness
# For skia
RUN ln -s /usr/lib/aarch64-linux-gnu/libfontconfig.so.1 /usr/lib/libfontconfig.so
RUN ln -s /usr/lib/aarch64-linux-gnu/libfreetype.so.6 /usr/lib/libfreetype.so
# For ffmpeg
# RUN ln -s /usr/lib/aarch64-linux-gnu/libavfilter.so /usr/lib/libavfilter.so
# RUN ln -s /usr/lib/aarch64-linux-gnu/libavdevice.so /usr/lib/libavdevice.so
# RUN ln -s /usr/include/aarch64-linux-gnu/libav* /usr/include/
# RUN ln -s /usr/include/aarch64-linux-gnu/libsw* /usr/include/
# RUN ln -s /usr/include/aarch64-linux-gnu/libav* /usr/aarch64-linux-gnu/include/
# RUN ln -s /usr/include/aarch64-linux-gnu/libsw* /usr/aarch64-linux-gnu/include/

# Create user
RUN adduser rustuser
USER rustuser

# Install Rust
RUN curl --proto "=https" --tlsv1.2 --retry 3 -sSfL https://sh.rustup.rs | sh -s -- -y
RUN source "${HOME}"/.cargo/env

ENV PATH="/home/rustuser/.cargo/bin:${PATH}"

RUN rustup target add aarch64-unknown-linux-gnu

# run by mounting with -v PATH_TO_REPO:/sirgovan-rust/
WORKDIR /sirgovan-rust/

ENV SKIA_BINARIES_URL="file:///sirgovan-rust/binaries/skia-binaries-0.64-aarch64-linux-gnu.tar.gz"
ENV PKG_CONFIG_SYSROOT_DIR="/usr/aarch64-linux-gnu/"
ENV CARGO_TARGET_DIR="./target-docker"

ENTRYPOINT ["cargo", "build", "--target=aarch64-unknown-linux-gnu"]

