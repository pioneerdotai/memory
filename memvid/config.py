"""
Configuration defaults and constants for Memvid
"""

from typing import Dict, Any

# QR Code settings
QR_VERSION = 1  # 1-40, higher = more data capacity
QR_ERROR_CORRECTION = 'M'  # L, M, Q, H
QR_BOX_SIZE = 10     # QR_BOX_SIZE * QR_VERSION dimensions (1 = 21 x 21, 20 = 97 x 97, 40 = 177×177) must be < frame height/width
QR_BORDER = 4
QR_FILL_COLOR = "black"
QR_BACK_COLOR = "white"

# Video settings
VIDEO_FPS = 30
VIDEO_CODEC = 'mp4v'
FRAME_WIDTH = 512
FRAME_HEIGHT = 512
VIDEO_FILE_TYPE = ".mp4"

# compression settings for QR codes
VIDEO_CRF = 28  # Constant Rate Factor (0-51, lower = better quality, 18 is visually lossless)
VIDEO_PRESET = 'slow'  # ultrafast, superfast, veryfast, faster, fast, medium, slow, slower, veryslow
VIDEO_PROFILE = 'baseline'  # baseline, main, high (baseline for max compatibility)

# Chunking settings - SIMPLIFIED
DEFAULT_CHUNK_SIZE = 512
DEFAULT_OVERLAP = 32

# Retrieval settings
DEFAULT_TOP_K = 5
BATCH_SIZE = 100
MAX_WORKERS = 4  # For parallel processing
CACHE_SIZE = 1000  # Number of frames to cache

# Embedding settings
EMBEDDING_MODEL = "all-MiniLM-L6-v2"  # Fast and good quality
EMBEDDING_DIMENSION = 384

# Index settings
INDEX_TYPE = "Flat"  # Can be "IVF" for larger datasets
NLIST = 100  # Number of clusters for IVF index

# LLM settings
DEFAULT_LLM_PROVIDER = "google"  # google, openai, anthropic
LLM_MODELS = {
    "google": "gemini-2.0-flash-exp",
    "openai": "gpt-4",
    "anthropic": "claude-3-5-sonnet-20241022"
}

MAX_TOKENS = 8192
TEMPERATURE = 0.1
CONTEXT_WINDOW = 32000

# Chat settings
MAX_HISTORY_LENGTH = 10
CONTEXT_CHUNKS_PER_QUERY = 5

# Performance settings
PREFETCH_FRAMES = 50
DECODE_TIMEOUT = 10  # seconds

def get_default_config() -> Dict[str, Any]:
    """Get default configuration dictionary"""
    return {
        "qr": {
            "version": QR_VERSION,
            "error_correction": QR_ERROR_CORRECTION,
            "box_size": QR_BOX_SIZE,
            "border": QR_BORDER,
            "fill_color": QR_FILL_COLOR,
            "back_color": QR_BACK_COLOR,
        },
        "video": {
            "fps": VIDEO_FPS,
            "codec": VIDEO_CODEC,
            "frame_width": FRAME_WIDTH,
            "frame_height": FRAME_HEIGHT,
            "crf": VIDEO_CRF,
            "preset": VIDEO_PRESET,
            "profile": VIDEO_PROFILE,
            "file_type": VIDEO_FILE_TYPE,
        },
        "chunking": {
            "chunk_size": DEFAULT_CHUNK_SIZE,
            "overlap": DEFAULT_OVERLAP,
        },
        "retrieval": {
            "top_k": DEFAULT_TOP_K,
            "batch_size": BATCH_SIZE,
            "max_workers": MAX_WORKERS,
            "cache_size": CACHE_SIZE,
        },
        "embedding": {
            "model": EMBEDDING_MODEL,
            "dimension": EMBEDDING_DIMENSION,
        },
        "index": {
            "type": INDEX_TYPE,
            "nlist": NLIST,
        },
        "llm": {
            "model": LLM_MODELS[DEFAULT_LLM_PROVIDER],
            "max_tokens": MAX_TOKENS,
            "temperature": TEMPERATURE,
            "context_window": CONTEXT_WINDOW,
        },
        "chat": {
            "max_history": MAX_HISTORY_LENGTH,
            "context_chunks": CONTEXT_CHUNKS_PER_QUERY,
        },
        "performance": {
            "prefetch_frames": PREFETCH_FRAMES,
            "decode_timeout": DECODE_TIMEOUT,
        }
    }