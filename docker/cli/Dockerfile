# Memvid CLI Docker Image
# Using Ubuntu base for newer GLIBC (required by memvid-cli binary)
FROM ubuntu:24.04

# Install Node.js and npm
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends \
    nodejs \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

LABEL org.opencontainers.image.title="Memvid CLI"
LABEL org.opencontainers.image.description="AI memory CLI with semantic search"
LABEL org.opencontainers.image.url="https://memvid.com"
LABEL org.opencontainers.image.source="https://github.com/memvid/memvid"
LABEL org.opencontainers.image.vendor="Memvid"
LABEL org.opencontainers.image.version="2.0.116"

# Install system dependencies required by memvid-cli binary
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Install memvid-cli globally
RUN npm install -g memvid-cli@latest

# Create a working directory for memory files
WORKDIR /data

# Set the entrypoint to memvid
ENTRYPOINT ["memvid"]

# Default command shows help
CMD ["--help"]
