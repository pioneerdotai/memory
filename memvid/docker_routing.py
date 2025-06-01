"""
Docker Backend Routing for Memvid
Handles Docker container management and codec routing logic.
"""

import os
import json
import subprocess
import tempfile
import shutil
from pathlib import Path
from typing import List, Optional, Dict, Any
import warnings

class DockerBackend:
    """Manages Docker backend for advanced video codecs."""

    # Codecs that benefit from Docker backend
    DOCKER_CODECS = {
        'h265', 'hevc', 'libx265',
        'h264', 'avc', 'libx264',
        'av1', 'libaom-av1'
    }

    def __init__(self, container_name="memvid", verbose=True):
        self.container_name = container_name
        self.verbose = verbose
        self.docker_cmd = None
        self.container_available = False
        self.setup_status = "unknown"
        self._check_docker_setup()

    def should_use_docker(self, codec: str) -> bool:
        """Check if codec should use Docker backend."""
        return codec.lower() in self.DOCKER_CODECS

    def is_available(self) -> bool:
        """Check if Docker backend is ready to use."""
        return self.container_available

    def get_status_message(self) -> str:
        """Get human-readable status message."""
        status = self.setup_status

        if status == "ready":
            return "✅ Docker backend ready for advanced codecs (H.265, H.264, VP9)"
        elif status == "container_missing":
            return "⚠️  Docker available but memvid-h265 container missing"
        elif status == "no_docker":
            return "ℹ️  Docker not found - using native encoding only"
        elif status == "docker_not_running":
            return "⚠️  Docker installed but not running"
        else:
            return "⚠️  Docker setup unclear - will attempt fallback if needed"

    def _check_docker_setup(self):
        """Check Docker availability and container status."""

        # Step 1: Find Docker command
        if shutil.which("docker.exe"):
            self.docker_cmd = "docker.exe"
        elif shutil.which("docker"):
            self.docker_cmd = "docker"
        else:
            self.setup_status = "no_docker"
            return

        # Step 2: Test Docker is running
        try:
            result = subprocess.run([self.docker_cmd, "--version"],
                                    capture_output=True, timeout=5)
            if result.returncode != 0:
                self.setup_status = "docker_not_running"
                return
        except (subprocess.TimeoutExpired, FileNotFoundError):
            self.setup_status = "docker_not_running"
            return

        # Step 3: Check if our container exists
        try:
            result = subprocess.run([self.docker_cmd, "images", "-q", self.container_name],
                                    capture_output=True, timeout=10)
            if result.returncode == 0 and result.stdout.strip():
                self.container_available = True
                self.setup_status = "ready"
            else:
                self.setup_status = "container_missing"
        except subprocess.TimeoutExpired:
            self.setup_status = "docker_slow"

    def ensure_container(self, auto_build=False, project_root=None):
        """Ensure Docker container is available, optionally building it."""

        if self.container_available:
            return True

        if self.setup_status != "container_missing":
            return False

        if not auto_build:
            if self.verbose:
                print(f"🐳 Docker container '{self.container_name}' not found")
                print("   Set auto_build=True to build automatically, or run 'make build'")
            return False

        # Try to build the container
        return self._build_container(project_root)

    def _build_container(self, project_root=None):
        """Build the Docker container."""
        try:
            if not project_root:
                project_root = self._find_project_root()

            if not project_root:
                if self.verbose:
                    print("❌ Cannot find project root with docker/ directory")
                return False

            if self.verbose:
                print(f"🏗️  Building {self.container_name} container...")

            dockerfile_path = project_root / "docker"

            cmd = [self.docker_cmd, "build", "-f",
                   str(dockerfile_path / "Dockerfile"),
                   "-t", self.container_name,
                   str(dockerfile_path)]

            result = subprocess.run(cmd, capture_output=True, text=True, timeout=300)

            if result.returncode == 0:
                if self.verbose:
                    print("✅ Container built successfully")
                self.container_available = True
                self.setup_status = "ready"
                return True
            else:
                if self.verbose:
                    print(f"❌ Container build failed: {result.stderr}")
                return False

        except subprocess.TimeoutExpired:
            if self.verbose:
                print("❌ Container build timed out")
            return False
        except Exception as e:
            if self.verbose:
                print(f"❌ Container build error: {e}")
            return False

    def encode_chunks(self, chunks: List[str], output_path: str,
                      project_root=None, **kwargs) -> Dict[str, Any]:
        """Encode chunks using Docker backend."""

        if not self.container_available:
            raise RuntimeError("Docker container not available")

        if not project_root:
            project_root = self._find_project_root()

        if not project_root:
            raise RuntimeError("Cannot find project root with docker/ directory")

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)

            # Create input file
            input_file = temp_path / "chunks.json"
            with open(input_file, 'w', encoding='utf-8') as f:
                json.dump(chunks, f, ensure_ascii=False)

            # Prepare output paths
            output_path = Path(output_path)
            temp_output = temp_path / "output.mp4"
            temp_index = temp_path / "output.json"

            # Convert paths for Docker volume mounting
            temp_str, scripts_path = self._prepare_docker_paths(temp_path, project_root)

            # Run Docker encoding
            cmd = [
                self.docker_cmd, "run", "--rm",
                "-v", f"{temp_str}:/data",
                "-v", f"{scripts_path}:/scripts",
                self.container_name,
                "python3", "/scripts/dockerized_encoder.py",
                "chunks.json", "output.mp4"
            ]

            if self.verbose:
                print(f"🎬 Encoding {len(chunks)} chunks with Docker backend...")

            result = subprocess.run(cmd, capture_output=True, text=True, timeout=3600)

            if result.returncode != 0:
                raise RuntimeError(f"Docker encoding failed: {result.stderr}")

            # Copy results back
            shutil.copy2(temp_output, output_path)

            # Handle index file if it exists
            index_path = output_path.with_suffix('.json')
            if temp_index.exists():
                shutil.copy2(temp_index, index_path)

            # Calculate stats
            file_size = output_path.stat().st_size
            file_size_mb = file_size / (1024 * 1024)

            return {
                "backend": "docker",
                "codec": "h265",
                "file_size_mb": round(file_size_mb, 2),
                "chunks": len(chunks),
                "chunks_per_mb": round(len(chunks) / file_size_mb, 0) if file_size_mb > 0 else 0
            }

    def _prepare_docker_paths(self, temp_path, project_root):
        """Prepare paths for Docker volume mounting."""

        # Check if we're in WSL or Windows
        if os.name == 'nt' or self._is_wsl():
            # Windows or WSL - convert to Windows paths
            temp_str = str(temp_path).replace('/mnt/c', 'C:')
            if temp_str.startswith('/'):
                # If still Unix path, might be native WSL - try to convert
                temp_str = temp_str.replace('/', '\\')
                if not temp_str.startswith('C:'):
                    # Fallback to Unix path
                    temp_str = str(temp_path)

            scripts_str = str(project_root / "docker" / "scripts").replace('/mnt/c', 'C:')
            if scripts_str.startswith('/'):
                scripts_str = scripts_str.replace('/', '\\')
                if not scripts_str.startswith('C:'):
                    scripts_str = str(project_root / "docker" / "scripts")
        else:
            # Native Linux/Mac
            temp_str = str(temp_path)
            scripts_str = str(project_root / "docker" / "scripts")

        return temp_str, scripts_str

    def _is_wsl(self):
        """Check if running in WSL."""
        try:
            with open('/proc/version', 'r') as f:
                return 'microsoft' in f.read().lower()
        except:
            return False

    def _find_project_root(self):
        """Find project root directory containing docker/ folder."""
        current = Path(__file__).parent

        for _ in range(5):  # Don't search too far up
            if (current / "docker").exists():
                return current
            current = current.parent

        return None


class CodecRouter:
    """Routes codec requests to appropriate backend."""

    def __init__(self, verbose=True):
        self.docker_backend = DockerBackend(verbose=verbose)
        self.verbose = verbose

    def get_backend_for_codec(self, codec: str) -> str:
        """Determine which backend should handle this codec."""
        if self.docker_backend.should_use_docker(codec):
            return "docker"
        return "native"

    def show_setup_status(self):
        """Show current setup status to user."""
        if self.verbose:
            print(self.docker_backend.get_status_message())

    def route_encoding(self, chunks: List[str], output_path: str, index_path: str,
                       codec: str = "mp4v", auto_build_docker: bool = True,
                       native_encoder=None, **kwargs) -> Dict[str, Any]:
        """
        Route encoding to appropriate backend based on codec.

        Args:
            chunks: Text chunks to encode
            output_path: Output video path
            index_path: Output index path
            codec: Video codec
            auto_build_docker: Whether to auto-build Docker if needed
            native_encoder: Function to call for native encoding
            **kwargs: Additional encoding parameters
        """

        backend = self.get_backend_for_codec(codec)

        if backend == "docker":
            return self._route_to_docker(chunks, output_path, index_path, codec,
                                         auto_build_docker, native_encoder, **kwargs)
        else:
            return self._route_to_native(chunks, output_path, index_path, codec,
                                         native_encoder, **kwargs)

    def _route_to_docker(self, chunks, output_path, index_path, codec, auto_build, native_encoder, **kwargs):
        """Route to Docker backend with fallback."""

        # Ensure container is available
        if not self.docker_backend.ensure_container(auto_build=auto_build):
            return self._handle_docker_fallback(chunks, output_path, index_path, codec, native_encoder, **kwargs)

        # Try Docker encoding
        try:
            result = self.docker_backend.encode_chunks(chunks, output_path, **kwargs)

            # Docker creates its own index, but we might want to enhance it
            self._enhance_index(index_path, result, chunks)

            if self.verbose:
                print(f"✅ Docker encoding complete: {output_path}")

            return result

        except Exception as e:
            if self.verbose:
                warnings.warn(f"Docker encoding failed: {e}. Falling back to native.", UserWarning)
            return self._handle_docker_fallback(chunks, output_path, index_path, "mp4v", native_encoder, **kwargs)

    def _route_to_native(self, chunks, output_path, index_path, codec, native_encoder, **kwargs):
        """Route to native backend."""

        if self.verbose:
            print(f"🏠 Using native backend for {codec} encoding...")

        if not native_encoder:
            raise RuntimeError("Native encoder function not provided")

        # Call the native encoder
        result = native_encoder(chunks, output_path, index_path, codec, **kwargs)

        # Ensure result has backend info
        if isinstance(result, dict):
            result["backend"] = "native"
        else:
            result = {"backend": "native", "codec": codec}

        return result

    def _handle_docker_fallback(self, chunks, output_path, index_path, codec, native_encoder, **kwargs):
        """Handle fallback to native when Docker fails."""

        status = self.docker_backend.setup_status

        if status == "no_docker":
            message = f"Codec '{codec}' requires Docker but Docker is not installed. Install Docker Desktop or use 'mp4v'. Falling back to MP4V."
        elif status == "docker_not_running":
            message = f"Codec '{codec}' requires Docker but Docker is not running. Start Docker Desktop or use 'mp4v'. Falling back to MP4V."
        else:
            message = f"Codec '{codec}' requires Docker backend but setup failed. Falling back to MP4V."

        if self.verbose:
            warnings.warn(message, UserWarning)

        # Fall back to native MP4V
        return self._route_to_native(chunks, output_path, index_path, "mp4v", native_encoder, **kwargs)

    def _enhance_index(self, index_path, docker_result, chunks):
        """Enhance index with additional metadata."""
        try:
            # Load existing index if it exists
            if Path(index_path).exists():
                with open(index_path, 'r', encoding='utf-8') as f:
                    index_data = json.load(f)
            else:
                # Create basic index
                index_data = {
                    'version': '2.0',
                    'chunks': [{'index': i, 'text': chunk} for i, chunk in enumerate(chunks)]
                }

            # Add routing metadata
            index_data['backend'] = docker_result.get('backend', 'docker')
            index_data['routing'] = {
                'used_docker': True,
                'codec': docker_result.get('codec', 'h265'),
                'stats': {
                    'file_size_mb': docker_result.get('file_size_mb'),
                    'chunks_per_mb': docker_result.get('chunks_per_mb')
                }
            }

            # Save enhanced index
            with open(index_path, 'w', encoding='utf-8') as f:
                json.dump(index_data, f, indent=2, ensure_ascii=False)

        except Exception as e:
            if self.verbose:
                print(f"Warning: Could not enhance index: {e}")


# Convenience functions for easy integration
def create_codec_router(verbose=True) -> CodecRouter:
    """Create a codec router instance."""
    return CodecRouter(verbose=verbose)

def route_encoding(chunks: List[str], output_path: str, index_path: str,
                   codec: str = "mp4v", native_encoder=None, **kwargs) -> Dict[str, Any]:
    """
    Convenience function for routing encoding based on codec.

    Example:
        def my_native_encoder(chunks, output_path, index_path, codec, **kwargs):
            # Your existing encoding logic
            return {"codec": codec}

        result = route_encoding(chunks, "out.mp4", "out.json", "h265",
                              native_encoder=my_native_encoder)
    """
    router = CodecRouter()
    return router.route_encoding(chunks, output_path, index_path, codec,
                                 native_encoder=native_encoder, **kwargs)