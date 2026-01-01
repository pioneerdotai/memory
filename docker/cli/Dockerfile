# Memvid CLI Docker Image
FROM node:20-slim

LABEL org.opencontainers.image.title="Memvid CLI"
LABEL org.opencontainers.image.description="AI memory CLI with semantic search"
LABEL org.opencontainers.image.url="https://memvid.com"
LABEL org.opencontainers.image.source="https://github.com/memvid/memvid"
LABEL org.opencontainers.image.vendor="Memvid"
LABEL org.opencontainers.image.version="2.0.116"

# Install memvid-cli globally
RUN npm install -g memvid-cli@latest

# Create a working directory for memory files
WORKDIR /data

# Set the entrypoint to memvid
ENTRYPOINT ["memvid"]

# Default command shows help
CMD ["--help"]
