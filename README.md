# Memvid - Video-Based AI Memory 🧠📹

**The lightweight, game-changing solution for AI memory at scale**

[![PyPI version](https://badge.fury.io/py/memvid.svg)](https://pypi.org/project/memvid/)
[![Downloads](https://pepy.tech/badge/memvid)](https://pepy.tech/project/memvid)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Python 3.8+](https://img.shields.io/badge/python-3.8+-blue.svg)](https://www.python.org/downloads/)
[![Code style: black](https://img.shields.io/badge/code%20style-black-000000.svg)](https://github.com/psf/black)

Memvid revolutionizes AI memory management by encoding text data into videos, enabling **lightning-fast semantic search** across millions of text chunks with **sub-second retrieval times**. Unlike traditional vector databases that consume massive amounts of RAM and storage, Memvid compresses your knowledge base into compact video files while maintaining instant access to any piece of information.

## ✨ Key Features

- 🎥 **Video-as-Database**: Store millions of text chunks in a single MP4 file
- 🔍 **Semantic Search**: Find relevant content using natural language queries
- 💬 **Built-in Chat**: Conversational interface with context-aware responses
- 📚 **PDF Support**: Direct import and indexing of PDF documents
- 🚀 **Fast Retrieval**: Sub-second search across massive datasets
- 💾 **Efficient Storage**: 10x compression compared to traditional databases
- 🔌 **Pluggable LLMs**: Works with OpenAI, Anthropic, or local models
- 🌐 **Offline-First**: No internet required after video generation
- 🔧 **Simple API**: Get started with just 3 lines of code

## 🎯 Use Cases

- **📖 Digital Libraries**: Index thousands of books in a single video file
- **🎓 Educational Content**: Create searchable video memories of course materials
- **📰 News Archives**: Compress years of articles into manageable video databases
- **💼 Corporate Knowledge**: Build company-wide searchable knowledge bases
- **🔬 Research Papers**: Quick semantic search across scientific literature
- **📝 Personal Notes**: Transform your notes into a searchable AI assistant

## 🚀 Why Memvid?

### Game-Changing Innovation
- **Video as Database**: Store millions of text chunks in a single MP4 file
- **Instant Retrieval**: Sub-second semantic search across massive datasets
- **10x Storage Efficiency**: Video compression reduces memory footprint dramatically
- **Zero Infrastructure**: No database servers, just files you can copy anywhere
- **Offline-First**: Works completely offline once videos are generated

### Lightweight Architecture
- **Minimal Dependencies**: Core functionality in ~1000 lines of Python
- **CPU-Friendly**: Runs efficiently without GPU requirements
- **Portable**: Single video file contains your entire knowledge base
- **Streamable**: Videos can be streamed from cloud storage

## 📦 Installation

### Quick Install
```bash
pip install memvid
```

### For PDF Support
```bash
pip install memvid PyPDF2
```

### Recommended Setup (Virtual Environment)
```bash
# Create a new project directory
mkdir my-memvid-project
cd my-memvid-project

# Create virtual environment
python -m venv venv

# Activate it
# On macOS/Linux:
source venv/bin/activate
# On Windows:
venv\Scripts\activate

# Install memvid
pip install memvid

# For PDF support:
pip install PyPDF2
```

## 🎯 Quick Start

### Basic Usage
```python
from memvid import MemvidEncoder, MemvidChat

# Create video memory from text chunks
chunks = ["Important fact 1", "Important fact 2", "Historical event details", ...]
encoder = MemvidEncoder()
encoder.add_chunks(chunks)
encoder.build_video("memory.mp4", "memory_index.json")

# Chat with your memory
chat = MemvidChat("memory.mp4", "memory_index.json")
chat.start_session()
response = chat.chat("What do you know about historical events?")
print(response)
```

### Building Memory from Documents
```python
from memvid import MemvidEncoder
import os

# Load documents
encoder = MemvidEncoder(chunk_size=512, overlap=50)

# Add text files
for file in os.listdir("documents"):
    with open(f"documents/{file}", "r") as f:
        encoder.add_text(f.read(), metadata={"source": file})

# Build optimized video
encoder.build_video(
    "knowledge_base.mp4",
    "knowledge_index.json",
    fps=30,  # Higher FPS = more chunks per second
    frame_size=512  # Larger frames = more data per frame
)
```

### Advanced Search & Retrieval
```python
from memvid import MemvidRetriever

# Initialize retriever
retriever = MemvidRetriever("knowledge_base.mp4", "knowledge_index.json")

# Semantic search
results = retriever.search("machine learning algorithms", top_k=5)
for chunk, score in results:
    print(f"Score: {score:.3f} | {chunk[:100]}...")

# Get context window
context = retriever.get_context("explain neural networks", max_tokens=2000)
print(context)
```

### Interactive Chat Interface
```python
from memvid import MemvidInteractive

# Launch interactive chat UI
interactive = MemvidInteractive("knowledge_base.mp4", "knowledge_index.json")
interactive.run()  # Opens web interface at http://localhost:7860
```

### Complete Example: Chat with a PDF Book
```bash
# 1. Create a new directory and set up environment
mkdir book-chat-demo
cd book-chat-demo
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# 2. Install dependencies
pip install memvid PyPDF2

# 3. Create book_chat.py
cat > book_chat.py << 'EOF'
from memvid import MemvidEncoder, chat_with_memory
import os

# Your PDF file
book_pdf = "book.pdf"  # Replace with your PDF path

# Build video memory
encoder = MemvidEncoder()
encoder.add_pdf(book_pdf)
encoder.build_video("book_memory.mp4", "book_index.json")

# Chat with the book
api_key = os.getenv("OPENAI_API_KEY")  # Optional: for AI responses
chat_with_memory("book_memory.mp4", "book_index.json", api_key=api_key)
EOF

# 4. Run it
export OPENAI_API_KEY="your-api-key"  # Optional
python book_chat.py
```


## 🔧 API Reference

### MemvidEncoder
```python
encoder = MemvidEncoder(
    chunk_size=512,      # Characters per chunk
    overlap=50,          # Character overlap between chunks
    model_name='all-MiniLM-L6-v2'  # Sentence transformer model
)

# Methods
encoder.add_chunks(chunks: List[str], metadata: List[dict] = None)
encoder.add_text(text: str, metadata: dict = None)
encoder.build_video(video_path: str, index_path: str, fps: int = 30, qr_size: int = 512)
```

### MemvidRetriever
```python
retriever = MemvidRetriever(
    video_path: str,
    index_path: str,
    cache_size: int = 100  # Number of frames to cache
)

# Methods
results = retriever.search(query: str, top_k: int = 5)
context = retriever.get_context(query: str, max_tokens: int = 2000)
chunks = retriever.get_chunks_by_ids(chunk_ids: List[int])
```

### MemvidChat
```python
chat = MemvidChat(
    video_path: str,
    index_path: str,
    llm_backend: str = 'openai',  # 'openai', 'anthropic', 'local'
    model: str = 'gpt-4'
)

# Methods
chat.start_session(system_prompt: str = None)
response = chat.chat(message: str, stream: bool = False)
chat.clear_history()
chat.export_conversation(path: str)
```

## 🛠️ Advanced Configuration

### Custom Embeddings
```python
from sentence_transformers import SentenceTransformer

# Use custom embedding model
custom_model = SentenceTransformer('sentence-transformers/all-mpnet-base-v2')
encoder = MemvidEncoder(embedding_model=custom_model)
```

### Video Optimization
```python
# For maximum compression
encoder.build_video(
    "compressed.mp4",
    "index.json",
    fps=60,  # More frames per second
    frame_size=256,  # Smaller frames
    video_codec='h265',  # Better compression
    crf=28  # Compression quality (lower = better quality)
)
```

### Distributed Processing
```python
# Process large datasets in parallel
encoder = MemvidEncoder(n_workers=8)
encoder.add_chunks_parallel(massive_chunk_list)
```

## 🐛 Troubleshooting

### Common Issues

**ModuleNotFoundError: No module named 'memvid'**
```bash
# Make sure you're using the right Python
which python  # Should show your virtual environment path
# If not, activate your virtual environment:
source venv/bin/activate  # On Windows: venv\Scripts\activate
```

**ImportError: PyPDF2 is required for PDF support**
```bash
pip install PyPDF2
```

**OpenAI API Key Issues**
```bash
# Set your API key (get one at https://platform.openai.com)
export OPENAI_API_KEY="sk-..."  # macOS/Linux
# Or on Windows:
set OPENAI_API_KEY=sk-...
```

**Large PDF Processing**
```python
# For very large PDFs, use smaller chunk sizes
encoder = MemvidEncoder()
encoder.add_pdf("large_book.pdf", chunk_size=400, overlap=50)
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

```bash
# Run tests
pytest tests/

# Run with coverage
pytest --cov=memvid tests/

# Format code
black memvid/
```

## 🆚 Comparison with Traditional Solutions

| Feature | Memvid | Vector DBs | Traditional DBs |
|---------|--------|------------|-----------------|
| Storage Efficiency | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ |
| Setup Complexity | Simple | Complex | Complex |
| Semantic Search | ✅ | ✅ | ❌ |
| Offline Usage | ✅ | ❌ | ✅ |
| Portability | File-based | Server-based | Server-based |
| Scalability | Millions | Millions | Billions |
| Cost | Free | $$$$ | $$$ |

## 🗺️ Roadmap

- [ ] **v0.2.0** - Multi-language support
- [ ] **v0.3.0** - Real-time memory updates
- [ ] **v0.4.0** - Distributed video sharding
- [ ] **v0.5.0** - Audio and image support
- [ ] **v1.0.0** - Production-ready with enterprise features

## 📚 Examples

Check out the [examples/](examples/) directory for:
- Building memory from Wikipedia dumps
- Creating a personal knowledge base
- Multi-language support
- Real-time memory updates
- Integration with popular LLMs

## 🆘 Getting Help

- 📖 [Documentation](https://github.com/olow304/memvid/wiki) - Comprehensive guides
- 💬 [Discussions](https://github.com/olow304/memvid/discussions) - Ask questions
- 🐛 [Issue Tracker](https://github.com/olow304/memvid/issues) - Report bugs
- 🌟 [Show & Tell](https://github.com/olow304/memvid/discussions/categories/show-and-tell) - Share your projects

## 🔗 Links

- [GitHub Repository](https://github.com/olow304/memvid)
- [PyPI Package](https://pypi.org/project/memvid)
- [Changelog](https://github.com/olow304/memvid/releases)


## 📄 License

MIT License - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

Created by [Olow304](https://github.com/olow304) and the Memvid community.

Built with ❤️ using:
- [sentence-transformers](https://www.sbert.net/) - State-of-the-art embeddings for semantic search
- [OpenCV](https://opencv.org/) - Computer vision and video processing
- [qrcode](https://github.com/lincolnloop/python-qrcode) - QR code generation
- [FAISS](https://github.com/facebookresearch/faiss) - Efficient similarity search
- [PyPDF2](https://github.com/py-pdf/pypdf) - PDF text extraction

Special thanks to all contributors who help make Memvid better!

---

**Ready to revolutionize your AI memory management? Install Memvid and start building!** 🚀