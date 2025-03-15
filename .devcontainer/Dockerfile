FROM ubuntu:latest

# Prevent some interactive dialogs during package installs
ENV DEBIAN_FRONTEND=noninteractive

# Create a non-root user named 'openggs'
RUN useradd -ms /bin/bash openggs

# Switch to root for installations
USER root

# Install development packages: Clang, Clang-Tidy, SDL2, etc.
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        build-essential \
        clang \
        clang-tidy \
        clang-format-18 \
        lldb \
        gdb \
        curl \
        cmake \
        bash-completion \
        vim \
        git && \
    rm -rf /var/lib/apt/lists/* && \ 
        update-ca-certificates


# Optionally set Clang as the default compiler
ENV CC=clang
ENV CXX=clang++

# Switch to openggs user and set working directory
USER openggs
WORKDIR /home/openggs

# 4) Install Rust as openggs (it will go into /home/openggs/.cargo)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# 5) Update PATH for the rest of the Docker build
ENV PATH="/home/openggs/.cargo/bin:${PATH}"

# Verify that rustc is now found
RUN rustc --version

# Install Git prompt script for a nicer shell experience
# RUN curl -L https://raw.github.com/git/git/master/contrib/completion/git-prompt.sh > ~/.bash_git && \
#    echo "source ~/.bash_git" >> ~/.bashrc && \
#    sed -Ei 's/(PS1=.*)(\\\[\\033\[00m\\\]\\\$.*)/\1\\[\\033[01;33m\\]$(__git_ps1)\2/p' ~/.bashrc
