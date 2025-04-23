FROM rust:1.86-alpine

RUN rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-musl
RUN apk add sqlite gdb just musl-dev vim nasm patchelf bash git make sudo

RUN addgroup -S sudo && adduser -S user -G sudo -u 1000
RUN echo '%sudo ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers

USER user
WORKDIR /home/user

RUN mkdir target sqlite-build
