FROM rust:buster

RUN apt update
# clang for bindgen
RUN apt install -y clang
# dynamorio requirements
RUN apt install -y cmake g++ g++-multilib transfig imagemagick ghostscript git zlib1g-dev

RUN rustup component add rustfmt
