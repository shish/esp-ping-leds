FROM debian:bookworm
ENV DEBIAN_FRONTEND=noninteractive
ENV LC_ALL=C.UTF-8
ENV LANG=C.UTF-8

# Install dependencies
RUN apt-get update && apt-get install -y sudo git curl build-essential \
    python3 python3-venv python3-packaging python3-pip libclang1 clang \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/*

# Add users
RUN adduser --disabled-password --gecos "" esp
RUN echo esp ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/esp \
    && chmod 0440 /etc/sudoers.d/esp

# Install rustup
WORKDIR /home/esp
USER esp
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
    --default-toolchain nightly -y
ENV PATH=${PATH}:/home/esp/.cargo/bin
RUN cargo install ldproxy

CMD [ "/bin/bash" ]
