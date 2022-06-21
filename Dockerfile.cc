FROM arm64v8/ubuntu:impish

RUN apt -y update && apt -y upgrade \
        && apt -y install gcc-aarch64-linux-gnu curl

RUN cd /tmp && curl -O https://static.rust-lang.org/rustup/dist/aarch64-unknown-linux-gnu/rustup-init \
                             && chmod 755 rustup-init \
                             && /tmp/rustup-init -y \
                             && mkdir -p /build \
                             && mkdir -p /depbuild/src

ENV PATH="/bin:/sbin:/usr/bin:/usr/sbin:/usr/local/bin:/usr/local/sbin:/root/.cargo/bin"

COPY Cargo.toml /depbuild
COPY Cargo.lock /depbuild
COPY src/dummy.rs /depbuild/src
WORKDIR /depbuild
RUN sed -i 's/main.rs/dummy.rs/' Cargo.toml
RUN cargo build --release
RUN sed -i 's/dummy.rs/main.rs/' Cargo.toml

WORKDIR /build
