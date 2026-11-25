FROM rust:1-bookworm

ENV DEBIAN_FRONTEND=noninteractive

# Install Linux dependencies required for Bevy (audio, input, windowing, Vulkan/Mesa)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        pkg-config \
        libasound2-dev \
        libudev-dev \
        libx11-dev \
        libwayland-dev \
        libxkbcommon-dev \
        git && \
    rm -rf /var/lib/apt/lists/* && \
    rustup component add clippy rustfmt

RUN useradd -ms /bin/bash openggs
USER openggs
WORKDIR /home/openggs
