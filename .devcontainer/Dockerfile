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
        build-essential \
        clang \
        clang-tidy \
        lldb \
        gdb \
        curl \
        cmake \
        bash-completion \
        vim \
        git \
        libsdl2-dev \
        libsdl2-image-dev \
        libsdl2-mixer-dev && \
    rm -rf /var/lib/apt/lists/*

# Optionally set Clang as the default compiler
ENV CC=clang
ENV CXX=clang++

# Switch to openggs user and set working directory
USER openggs
WORKDIR /home/openggs

# Install Git prompt script for a nicer shell experience
# RUN curl -L https://raw.github.com/git/git/master/contrib/completion/git-prompt.sh > ~/.bash_git && \
#    echo "source ~/.bash_git" >> ~/.bashrc && \
#    sed -Ei 's/(PS1=.*)(\\\[\\033\[00m\\\]\\\$.*)/\1\\[\\033[01;33m\\]$(__git_ps1)\2/p' ~/.bashrc
