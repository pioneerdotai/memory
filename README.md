# Memvid - QR Code Video-Based AI Memory

A high-performance Python library for creating, storing, and retrieving AI memory using QR code videos. Memvid enables semantic search across millions of text chunks with sub-second retrieval times.

## Features

- **Fast Encoding**: Convert text chunks into QR code videos
- **Semantic Search**: Lightning-fast retrieval using sentence embeddings
- **Conversational AI**: Built-in chat interface with context-aware memory
- **Scalable**: Handles millions of chunks with < 2 second retrieval
- **Flexible**: Pluggable LLM backends (OpenAI, Claude, Gemini, local models)

## Installation

```bash
pip install memvid
```

## Quick Start

```python
from memvid import MemvidEncoder, MemvidChat

# Create video memory from text chunks
chunks = ["Important fact 1", "Important fact 2", ...]
encoder = MemvidEncoder()
encoder.add_chunks(chunks)
encoder.build_video("memory.mp4", "memory_index.json")

# Chat with your memory
chat = MemvidChat("memory.mp4", "memory_index.json")
chat.start_session()
response = chat.chat("What do you know about...")
```

## Requirements

- Python 3.8+
- System dependencies for pyzbar (libzbar0 on Ubuntu/Debian)

## License

MIT License