# Memvid Docker Makefile
.PHONY: help docker-build docker-test docker-pull docker-clean

# Auto-detect Docker command (WSL/Windows compatibility)
DOCKER_CMD := $(shell if command -v docker.exe >/dev/null 2>&1; then echo "docker.exe"; else echo "docker"; fi)

# Get absolute path and convert for Docker mounting if needed
PWD := $(shell pwd)
DOCKER_PWD := $(shell pwd | sed 's|^/mnt/c|C:|' | sed 's|^/mnt/\([a-z]\)|\U\1:|')

# Docker image settings
IMAGE_NAME := memvid/cli
IMAGE_TAG := latest

help:
	@echo "🐳 Memvid Docker Commands"
	@echo ""
	@echo "  make docker-build  - Build the CLI Docker image locally"
	@echo "  make docker-test    - Test the CLI Docker image"
	@echo "  make docker-pull    - Pull the latest image from Docker Hub"
	@echo "  make docker-clean   - Clean up Docker images"
	@echo ""

docker-build:
	@echo "🏗️  Building Memvid CLI Docker image..."
	$(DOCKER_CMD) build -f docker/cli/Dockerfile -t $(IMAGE_NAME):test docker/cli/
	@echo "✅ Build complete"

docker-pull:
	@echo "📥 Pulling latest image..."
	$(DOCKER_CMD) pull $(IMAGE_NAME):$(IMAGE_TAG)
	@echo "✅ Pull complete"

docker-test: docker-build
	@echo "🧪 Testing CLI Docker image..."
	@$(DOCKER_CMD) run --rm $(IMAGE_NAME):test --version
	@echo "✅ Test passed"

docker-clean:
	@echo "🧹 Cleaning up..."
	-$(DOCKER_CMD) rmi $(IMAGE_NAME):test 2>/dev/null || true
	@echo "✅ Cleanup complete"
