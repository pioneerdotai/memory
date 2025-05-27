from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="memvid",
    version="0.1.0",
    author="Memvid Team",
    author_email="team@memvid.ai",
    description="QR code video-based AI memory library for fast semantic search and retrieval",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/memvid-org/memvid",
    packages=find_packages(),
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "Topic :: Software Development :: Libraries :: Python Modules",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
    ],
    python_requires=">=3.8",
    install_requires=[
        "qrcode[pil]>=8.0",
        "opencv-python>=4.11.0",
        "pyzbar>=0.1.9",
        "sentence-transformers>=4.0.0",
        "numpy>=2.0.0",
        "openai>=1.0.0",
        "tqdm>=4.67.0",
        "faiss-cpu>=1.11.0",
        "Pillow>=11.0.0",
    ],
    extras_require={
        "dev": [
            "pytest>=7.0.0",
            "pytest-cov>=4.0.0",
            "black>=23.0.0",
            "flake8>=6.0.0",
        ],
        "web": [
            "fastapi>=0.100.0",
            "gradio>=4.0.0",
        ],
    },
)