FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge
SHELL ["/bin/bash", "-c"]

RUN dpkg --add-architecture arm64 && \
    apt-get update && apt install fontconfig:arm64 -y

RUN curl --proto "=https" --tlsv1.2 --retry 3 -sSfL https://sh.rustup.rs | sh -s -- -y
RUN source "${HOME}"/.cargo/env

ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup target add aarch64-unknown-linux-gnu

RUN ln -s /usr/lib/aarch64-linux-gnu/libfontconfig.so.1 /usr/lib/libfontconfig.so
RUN ln -s /usr/lib/aarch64-linux-gnu/libfreetype.so.6 /usr/lib/libfreetype.so

# run by mounting with -v PATH_TO_REPO:/sirgovan-rust/
WORKDIR /sirgovan-rust/

ENV SKIA_BINARIES_URL="file:///sirgovan-rust/binaries/skia-binaries-0.64-aarch64-linux-gnu.tar.gz"

ENTRYPOINT ["cargo", "build", "--target=aarch64-unknown-linux-gnu"]

