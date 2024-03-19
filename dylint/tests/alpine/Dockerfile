FROM alpine

COPY . dylint

WORKDIR dylint

RUN apk --no-cache add g++ pkgconfig openssl-dev openssl-libs-static

# smoelius: Based on: https://www.rust-lang.org/learn/get-started
RUN wget -O - https://sh.rustup.rs | sh -s -- -y

ENV PATH="/root/.cargo/bin:$PATH"

RUN cargo install --path dylint-link

RUN cargo build --all-targets

ENV RUSTFLAGS="-C target-feature=-crt-static"

RUN cargo test -p dylint --lib
