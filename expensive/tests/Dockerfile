FROM alpine

COPY . dylint

WORKDIR dylint

RUN apk --no-cache add g++ pkgconfig openssl-dev openssl-libs-static

# smoelius: Based on: https://www.rust-lang.org/learn/get-started
RUN wget -O - https://sh.rustup.rs | sh -s -- -y

ENV PATH="/root/.cargo/bin:$PATH"

# smoelius: The nightly toolchain is needed by the `can_install_while_driver_is_running` test.
RUN rustup install nightly
RUN rustup component add llvm-tools-preview rustc-dev --toolchain nightly

RUN cargo install cargo-nested
RUN cargo install --path dylint-link

RUN cargo nested fetch

# The musl target enables static CRT linking by default, which is incompatible with the example
# libraries' `cdylib` crate type. Disabling it allows those dynamic libraries to be built.
ENV RUSTFLAGS="-C target-feature=-crt-static"

RUN cargo build --all-targets

RUN cargo test -p dylint --lib
