FROM alpine

COPY . dylint

WORKDIR dylint

RUN apk --no-cache add g++ pkgconfig openssl-dev openssl-libs-static

# smoelius: Based on: https://www.rust-lang.org/learn/get-started
RUN wget -O - https://sh.rustup.rs | sh -s -- -y

ENV PATH="/root/.cargo/bin:$PATH"

# smoelius: The nightly toolchain is needed by the `can_install_while_driver_is_running` test.
RUN rustup install nightly --no-self-update 
RUN rustup component add llvm-tools-preview rustc-dev --toolchain nightly

RUN cargo install --path dylint-link

RUN cargo build --all-targets

ENV RUSTFLAGS="-C target-feature=-crt-static"

RUN cargo test -p dylint --lib
