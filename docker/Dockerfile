
FROM --platform=linux/amd64 ubuntu:22.04

# Prevent interactive prompts
ENV DEBIAN_FRONTEND=noninteractive

# Install everything needed
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    python3-dev \
    ffmpeg \
    libzbar0 \
    libzbar-dev \
    && rm -rf /var/lib/apt/lists/*

# Set up Python environment
ENV PYTHONUNBUFFERED=1
ENV PYTHONOPTIMIZE=2

# Install Python packages
COPY requirements-compute.txt /tmp/
RUN pip3 install --no-cache-dir --upgrade pip && \
    pip3 install --no-cache-dir -r /tmp/requirements-compute.txt

# Create working directory
WORKDIR /data

# That's it! No script copying, no entrypoints, just a Python environment
CMD ["python3", "--version"]