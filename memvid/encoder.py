"""
MemvidEncoder - Handles chunking and QR video creation with Docker H.265 support
"""

import json
import logging
import subprocess
import shutil
import tempfile
import warnings
from pathlib import Path
from typing import List, Optional, Dict, Any
from tqdm import tqdm
import cv2
import numpy as np

from .utils import encode_to_qr, qr_to_frame, create_video_writer, chunk_text
from .index import IndexManager
from .config import get_default_config, DEFAULT_CHUNK_SIZE, DEFAULT_OVERLAP

logger = logging.getLogger(__name__)

class MemvidEncoder:
    """
    Enhanced MemvidEncoder with Docker backend support for H.265 encoding.
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None, enable_docker=True):
        self.config = config or get_default_config()
        self.chunks = []
        self.index_manager = IndexManager()

        # Docker backend detection
        self.enable_docker = enable_docker
        self._docker_cmd = None
        self._docker_available = False
        self._check_docker_setup()
        self.video_file_type = self.config.get("codect_parameters").get("video_file_type")

    def _check_docker_setup(self):
        """Check if Docker backend is available."""
        if not self.enable_docker:
            return

        # Find Docker command
        if shutil.which("docker.exe"):
            self._docker_cmd = "docker.exe"
        elif shutil.which("docker"):
            self._docker_cmd = "docker"
        else:
            return

        # Test if memvid-h265 container exists
        try:
            result = subprocess.run([self._docker_cmd, "images", "-q", "memvid-h265"],
                                    capture_output=True, timeout=5, encoding='utf-8', errors='replace')
            if result.returncode == 0 and result.stdout.strip():
                self._docker_available = True
                logger.info("✅ Docker H.265 backend available")
            else:
                logger.info("⚠️  Docker found but memvid-h265 container missing (run 'make build')")
        except:
            pass

    def _should_use_docker(self, codec):
        """Check if codec should use Docker backend."""
        docker_codecs = {'h265', 'hevc', 'libx265', 'h264', 'avc', 'libx264', 'vp9', 'av1'}
        return codec.lower() in docker_codecs

    def _find_project_root(self):
        """Find project root directory containing docker/ folder."""
        current = Path(__file__).parent
        for _ in range(5):
            if (current / "docker").exists():
                return current
            current = current.parent
        return None

    def _prepare_docker_paths(self, temp_path, project_root):
        """Convert paths for Docker volume mounting."""
        temp_str = str(temp_path)
        scripts_str = str(project_root / "docker" / "scripts")

        # Convert /mnt/c paths to Windows format for Docker Desktop
        if temp_str.startswith('/mnt/c'):
            temp_str = temp_str.replace('/mnt/c', 'C:')
        if scripts_str.startswith('/mnt/c'):
            scripts_str = scripts_str.replace('/mnt/c', 'C:')

        return temp_str, scripts_str

    def _build_with_docker(self, output_file, index_file, show_progress=True, **kwargs):
        """Build video using Docker H.265 backend."""

        if not self._docker_available:
            raise RuntimeError("Docker backend not available")

        # Find project root
        project_root = self._find_project_root()
        if not project_root:
            raise RuntimeError("Cannot find project root with docker/ directory")

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)

            # Create input subdirectory to match Docker script expectations
            input_dir = temp_path / "input"
            output_dir = temp_path / "output"
            input_dir.mkdir()
            output_dir.mkdir()

            # Create input file in input subdirectory
            input_file = input_dir / "chunks.json"
            with open(input_file, 'w', encoding='utf-8') as f:
                json.dump(self.chunks, f, ensure_ascii=False)

            # Prepare Docker paths
            temp_str, scripts_path = self._prepare_docker_paths(temp_path, project_root)

            # Run Docker encoding
            cmd = [
                self._docker_cmd, "run", "--rm",
                "-v", f"{temp_str}:/data",
                "-v", f"{scripts_path}:/scripts",
                "memvid-h265",
                "python3", "/scripts/dockerized_encoder.py",
                "chunks.json", f"output.{self.video_file_type}"
            ]

            if show_progress:
                print(f"🐳 Encoding {len(self.chunks)} chunks with Docker H.265...")

            result = subprocess.run(cmd, capture_output=True, text=True, timeout=3600, encoding='utf-8', errors='replace')

            if result.returncode != 0:
                raise RuntimeError(f"Docker encoding failed: {result.stderr}")

            # Copy results back
            temp_output = output_dir / f"output.{self.video_file_type}"
            temp_index = output_dir / "output.json"

            shutil.copy2(temp_output, output_file)
            if temp_index.exists():
                shutil.copy2(temp_index, index_file)

    def _try_build_container(self):
        """Try to build the Docker container."""
        try:
            project_root = self._find_project_root()
            if not project_root:
                return False

            dockerfile_path = project_root / "docker"
            cmd = [self._docker_cmd, "build", "-f", str(dockerfile_path / "Dockerfile"),
                   "-t", "memvid-h265", str(dockerfile_path)]

            print("🏗️  Building memvid-h265 container...")
            result = subprocess.run(cmd, capture_output=True, timeout=300, encoding='utf-8', errors='replace')
            if result.returncode == 0:
                self._docker_available = True
                print("✅ Docker container built successfully")
                return True
            else:
                print("❌ Docker container build failed")
                return False
        except:
            return False

    def add_chunks(self, chunks: List[str]):
        """
        Add text chunks to be encoded

        Args:
            chunks: List of text chunks
        """
        self.chunks.extend(chunks)
        logger.info(f"Added {len(chunks)} chunks. Total: {len(self.chunks)}")

    def add_text(self, text: str, chunk_size: int = DEFAULT_CHUNK_SIZE, overlap: int = DEFAULT_OVERLAP):
        """
        Add text and automatically chunk it

        Args:
            text: Text to chunk and add
            chunk_size: Target chunk size
            overlap: Overlap between chunks
        """
        chunks = chunk_text(text, chunk_size, overlap)
        self.add_chunks(chunks)

    def add_pdf(self, pdf_path: str, chunk_size: int = DEFAULT_CHUNK_SIZE, overlap: int = DEFAULT_OVERLAP):
        """
        Extract text from PDF and add as chunks

        Args:
            pdf_path: Path to PDF file
            chunk_size: Target chunk size (default larger for books)
            overlap: Overlap between chunks
        """
        try:
            import PyPDF2
        except ImportError:
            raise ImportError("PyPDF2 is required for PDF support. Install with: pip install PyPDF2")

        if not Path(pdf_path).exists():
            raise FileNotFoundError(f"PDF file not found: {pdf_path}")

        text = ""
        with open(pdf_path, 'rb') as file:
            pdf_reader = PyPDF2.PdfReader(file)
            num_pages = len(pdf_reader.pages)

            logger.info(f"Extracting text from {num_pages} pages of {Path(pdf_path).name}")

            for page_num in range(num_pages):
                page = pdf_reader.pages[page_num]
                page_text = page.extract_text()
                text += page_text + "\n\n"

        if text.strip():
            self.add_text(text, chunk_size, overlap)
            logger.info(f"Added PDF content: {len(text)} characters from {Path(pdf_path).name}")
        else:
            logger.warning(f"No text extracted from PDF: {pdf_path}")

    def add_epub(self, epub_path: str, chunk_size: int = DEFAULT_CHUNK_SIZE, overlap: int = DEFAULT_OVERLAP):
        """
        Extract text from EPUB and add as chunks

        Args:
            epub_path: Path to EPUB file
            chunk_size: Target chunk size (default larger for books)
            overlap: Overlap between chunks
        """
        try:
            import ebooklib
            from ebooklib import epub
            from bs4 import BeautifulSoup
        except ImportError:
            raise ImportError("ebooklib and beautifulsoup4 are required for EPUB support. Install with: pip install ebooklib beautifulsoup4")

        if not Path(epub_path).exists():
            raise FileNotFoundError(f"EPUB file not found: {epub_path}")

        try:
            book = epub.read_epub(epub_path)
            text_content = []

            logger.info(f"Extracting text from EPUB: {Path(epub_path).name}")

            # Extract text from all document items
            for item in book.get_items():
                if item.get_type() == ebooklib.ITEM_DOCUMENT:
                    # Parse HTML content
                    soup = BeautifulSoup(item.get_content(), 'html.parser')

                    # Remove script and style elements
                    for script in soup(["script", "style"]):
                        script.decompose()

                    # Get text and clean it up
                    text = soup.get_text()

                    # Clean up whitespace
                    lines = (line.strip() for line in text.splitlines())
                    chunks = (phrase.strip() for line in lines for phrase in line.split("  "))
                    text = ' '.join(chunk for chunk in chunks if chunk)

                    if text.strip():
                        text_content.append(text)

            # Combine all text
            full_text = "\n\n".join(text_content)

            if full_text.strip():
                self.add_text(full_text, chunk_size, overlap)
                logger.info(f"Added EPUB content: {len(full_text)} characters from {Path(epub_path).name}")
            else:
                logger.warning(f"No text extracted from EPUB: {epub_path}")

        except Exception as e:
            logger.error(f"Error processing EPUB {epub_path}: {e}")
            raise

    def build_video(self, output_file: str, index_file: str,
                    codec: str = "mp4v", show_progress: bool = True,
                    auto_build_docker: bool = True, allow_fallback: bool = True) -> Dict[str, Any]:
        """
        Build QR code video and index from chunks with smart codec routing

        Args:
            output_file: Path to output video file
            index_file: Path to output index file
            codec: Video codec ('mp4v', 'h265', 'h264', etc.)
            show_progress: Show progress bar
            auto_build_docker: Whether to auto-build Docker if needed
            allow_fallback: Whether to fall back to MP4V if Docker fails

        Returns:
            Dictionary with build statistics
        """
        if not self.chunks:
            raise ValueError("No chunks to encode. Use add_chunks() first.")

        output_path = Path(output_file)
        index_path = Path(index_file)

        # Ensure output directory exists
        output_path.parent.mkdir(parents=True, exist_ok=True)
        index_path.parent.mkdir(parents=True, exist_ok=True)

        logger.info(f"Building video with {len(self.chunks)} chunks using {codec} codec")

        # Route based on codec
        if self._should_use_docker(codec):

            # Try Docker backend
            if self._docker_available:
                try:
                    self._build_with_docker(output_file, index_file, show_progress)

                    # Get file stats for return
                    stats = {
                        "backend": "docker",
                        "codec": codec,
                        "total_chunks": len(self.chunks),
                        "video_file": str(output_path),
                        "index_file": str(index_path),
                        "video_size_mb": output_path.stat().st_size / (1024 * 1024) if output_path.exists() else 0,
                    }

                    if show_progress:
                        print(f"✅ H.265 encoding complete: {output_path}")
                        print(f"   File size: {stats['video_size_mb']:.2f} MB")

                    return stats

                except Exception as e:
                    if allow_fallback:
                        warnings.warn(f"Docker encoding failed: {e}. Falling back to MP4V.", UserWarning)
                    else:
                        raise RuntimeError(f"Docker H.265 encoding failed: {e}")

            # Docker not available or failed - try to build
            elif auto_build_docker and self._docker_cmd:
                if self._try_build_container():
                    try:
                        self._build_with_docker(output_file, index_file, show_progress)

                        stats = {
                            "backend": "docker",
                            "codec": codec,
                            "total_chunks": len(self.chunks),
                            "video_file": str(output_path),
                            "index_file": str(index_path),
                            "video_size_mb": output_path.stat().st_size / (1024 * 1024) if output_path.exists() else 0,
                        }

                        if show_progress:
                            print(f"✅ H.265 encoding complete: {output_path}")

                        return stats

                    except Exception as e:
                        if allow_fallback:
                            warnings.warn(f"Docker encoding failed after build: {e}. Falling back to MP4V.", UserWarning)
                        else:
                            raise RuntimeError(f"Docker H.265 encoding failed after build: {e}")

            # Show helpful message and either fall back or fail
            if not allow_fallback:
                if not self._docker_cmd:
                    raise RuntimeError(f"Codec '{codec}' requires Docker but Docker not found. Install Docker Desktop.")
                else:
                    raise RuntimeError(f"Codec '{codec}' requires Docker backend. Run 'make build' first.")

            # Fallback warnings
            if not self._docker_cmd:
                warnings.warn(f"Codec '{codec}' requires Docker but Docker not found. Install Docker Desktop. Using MP4V fallback.", UserWarning)
            else:
                warnings.warn(f"Codec '{codec}' requires Docker backend. Run 'make build' or use MP4V. Using MP4V fallback.", UserWarning)

            # Fall back to native MP4V
            codec = "mp4v"

        # Native encoding
        if show_progress:
            print(f"🏠 Using native OpenCV encoding for {codec}...")

        # Create video writer
        video_config = self.config.get("codec_parameters")
        writer = create_video_writer(str(output_path), video_config)

        frame_numbers = []

        try:
            # Generate QR codes and write to video
            chunks_iter = enumerate(self.chunks)
            if show_progress:
                chunks_iter = tqdm(chunks_iter, total=len(self.chunks), desc="Encoding chunks to video")

            for frame_num, chunk in chunks_iter:
                # Create metadata for chunk
                chunk_data = {
                    "id": frame_num,
                    "text": chunk,
                    "frame": frame_num
                }

                # Encode to QR
                qr_image = encode_to_qr(json.dumps(chunk_data))

                # Convert to video frame
                frame = qr_to_frame(qr_image, (video_config["frame_width"], video_config["frame_height"]))

                # Write frame
                writer.write(frame)
                frame_numbers.append(frame_num)

            # Add chunks to index
            if show_progress:
                logger.info("Building search index...")
            self.index_manager.add_chunks(self.chunks, frame_numbers, show_progress)

            # Save index
            self.index_manager.save(str(index_path.with_suffix('')))

            # Get statistics
            stats = {
                "backend": "native",
                "codec": codec,
                "total_chunks": len(self.chunks),
                "total_frames": len(frame_numbers),
                "video_file": str(output_path),
                "index_file": str(index_path),
                "video_size_mb": output_path.stat().st_size / (1024 * 1024) if output_path.exists() else 0,
                "fps": video_config["video_fps"],
                "duration_seconds": len(frame_numbers) / video_config["video_fps"],
                "index_stats": self.index_manager.get_stats()
            }

            if show_progress:
                logger.info(f"Successfully built video: {output_path}")
                logger.info(f"Video duration: {stats['duration_seconds']:.1f} seconds")
                logger.info(f"Video size: {stats['video_size_mb']:.1f} MB")

            return stats

        finally:
            writer.release()

    def clear(self):
        """Clear all chunks"""
        self.chunks = []
        self.index_manager = IndexManager()
        logger.info("Cleared all chunks")

    def get_stats(self) -> Dict[str, Any]:
        """Get encoder statistics"""
        return {
            "total_chunks": len(self.chunks),
            "total_characters": sum(len(chunk) for chunk in self.chunks),
            "avg_chunk_size": np.mean([len(chunk) for chunk in self.chunks]) if self.chunks else 0,
            "docker_available": self._docker_available,
            "config": self.config
        }

    def get_docker_status(self) -> str:
        """Get Docker backend status message."""
        if not self.enable_docker:
            return "Docker backend disabled"
        elif self._docker_available:
            return "✅ Docker H.265 backend ready"
        elif self._docker_cmd:
            return "⚠️  Docker found but memvid-h265 container missing"
        else:
            return "ℹ️  Docker not found - native encoding only"

    @classmethod
    def from_file(cls, file_path: str, chunk_size: int = DEFAULT_CHUNK_SIZE, overlap: int = DEFAULT_OVERLAP,
                  config: Optional[Dict[str, Any]] = None) -> 'MemvidEncoder':
        """
        Create encoder from text file

        Args:
            file_path: Path to text file
            chunk_size: Target chunk size
            overlap: Overlap between chunks
            config: Optional configuration

        Returns:
            MemvidEncoder instance with chunks loaded
        """
        encoder = cls(config)

        with open(file_path, 'r', encoding='utf-8') as f:
            text = f.read()

        encoder.add_text(text, chunk_size, overlap)
        return encoder

    @classmethod
    def from_documents(cls, documents: List[str], chunk_size: int = DEFAULT_CHUNK_SIZE, overlap: int = DEFAULT_OVERLAP,
                       config: Optional[Dict[str, Any]] = None) -> 'MemvidEncoder':
        """
        Create encoder from list of documents

        Args:
            documents: List of document strings
            chunk_size: Target chunk size
            overlap: Overlap between chunks
            config: Optional configuration

        Returns:
            MemvidEncoder instance with chunks loaded
        """
        encoder = cls(config)

        for doc in documents:
            encoder.add_text(doc, chunk_size, overlap)

        return encoder
