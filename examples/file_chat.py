#!/usr/bin/env python3
"""
file_chat.py - Example script for testing MemvidChat with external files

This script allows you to:
1. Create a memory video from your own files
2. Chat with the created memory using different LLM providers
3. Store results in output/ directory to avoid contaminating the main repo

Usage:
    python file_chat.py --input-dir /path/to/documents --provider google
    python file_chat.py --files file1.txt file2.pdf --provider openai
    python file_chat.py --load-existing output/my_memory --provider google

Examples:
    # Create memory from a directory and chat with Google
    python file_chat.py --input-dir ~/Documents/research --provider google

    # Create memory from specific files and chat with OpenAI
    python file_chat.py --files report.pdf notes.txt --provider openai

    # Load existing memory and continue chatting
    python file_chat.py --load-existing output/research_memory --provider google
"""

import argparse
import os
import sys
import time
from pathlib import Path
from datetime import datetime
import json

# Add the parent directory to the path so we can import memvid
sys.path.insert(0, str(Path(__file__).parent.parent))  # Go up TWO levels from examples/

from memvid import MemvidEncoder, MemvidChat
from memvid.config import get_default_config, VIDEO_FILE_TYPE


def setup_output_dir():
    """Create output directory if it doesn't exist"""
    output_dir = Path("output")
    output_dir.mkdir(exist_ok=True)
    return output_dir

def generate_memory_name(input_source):
    """Generate a meaningful name for the memory files"""
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")

    if isinstance(input_source, list):
        # Multiple files
        base_name = f"files_{len(input_source)}items"
    else:
        # Directory
        dir_name = Path(input_source).name
        base_name = f"dir_{dir_name}"

    return f"{base_name}_{timestamp}"

def collect_files_from_directory(directory_path, extensions=None):
    """Collect supported files from a directory"""
    if extensions is None:
        extensions = {'.txt', '.md', '.pdf', '.doc', '.docx', '.rtf', '.epub', '.html', '.htm'}

    directory = Path(directory_path)
    if not directory.exists():
        raise ValueError(f"Directory does not exist: {directory_path}")

    files = []
    for ext in extensions:
        files.extend(directory.rglob(f"*{ext}"))

    return [str(f) for f in files if f.is_file()]


def create_memory_from_files(files, output_dir, memory_name):
    """Create a memory video from a list of files"""
    print(f"Creating memory from {len(files)} files...")

    # Start timing
    start_time = time.time()

    # Get chunking config
    config = get_default_config()
    chunk_size = config["chunking"]["chunk_size"]  # Now 1024
    overlap = config["chunking"]["overlap"]        # Now 100

    print(f"Using chunk_size: {chunk_size}, overlap: {overlap}")

    # Initialize encoder
    encoder = MemvidEncoder()

    processed_count = 0
    skipped_count = 0

    for file_path in files:
        file_path = Path(file_path)
        print(f"Processing: {file_path.name}")

        try:
            if file_path.suffix.lower() == '.pdf':
                encoder.add_pdf(str(file_path), chunk_size, overlap)
            elif file_path.suffix.lower() == '.epub':
                encoder.add_epub(str(file_path), chunk_size, overlap)
            elif file_path.suffix.lower() in ['.html', '.htm']:
                # Process HTML with BeautifulSoup
                try:
                    from bs4 import BeautifulSoup
                except ImportError:
                    print(f"Warning: BeautifulSoup not available for HTML processing. Skipping {file_path.name}")
                    skipped_count += 1
                    continue

                with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                    soup = BeautifulSoup(f.read(), 'html.parser')
                    for script in soup(["script", "style"]):
                        script.decompose()
                    text = soup.get_text()
                    lines = (line.strip() for line in text.splitlines())
                    chunks = (phrase.strip() for line in lines for phrase in line.split("  "))
                    clean_text = ' '.join(chunk for chunk in chunks if chunk)
                    if clean_text.strip():
                        encoder.add_text(clean_text, chunk_size, overlap)
            else:
                # Read as text file
                with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                    content = f.read()
                    if content.strip():
                        encoder.add_text(content, chunk_size, overlap)

            processed_count += 1

        except Exception as e:
            print(f"Warning: Could not process {file_path.name}: {e}")
            skipped_count += 1
            continue

    processing_time = time.time() - start_time
    print(f"\n📊 Processing Summary:")
    print(f"  ✅ Successfully processed: {processed_count} files")
    print(f"  ⚠️  Skipped: {skipped_count} files")
    print(f"  ⏱️  Processing time: {processing_time:.2f} seconds")

    # Build the video
    video_path = output_dir / f"{memory_name}.{VIDEO_FILE_TYPE}"
    index_path = output_dir / f"{memory_name}_index.json"

    print(f"\n🎬 Building memory video: {video_path}")
    encoding_start = time.time()

    build_stats = encoder.build_video(str(video_path), str(index_path))

    encoding_time = time.time() - encoding_start
    total_time = time.time() - start_time

    # Enhanced statistics
    print(f"\n🎉 Memory created successfully!")
    print(f"  📁 Video: {video_path}")
    print(f"  📋 Index: {index_path}")
    print(f"  📊 Chunks: {build_stats.get('total_chunks', 'unknown')}")
    print(f"  🎞️  Frames: {build_stats.get('total_frames', 'unknown')}")
    print(f"  📏 Video size: {build_stats.get('video_size_mb', 0):.1f} MB")
    print(f"  ⏱️  Encoding time: {encoding_time:.2f} seconds")
    print(f"  ⏱️  Total time: {total_time:.2f} seconds")

    if build_stats.get('video_size_mb', 0) > 0:
        # Calculate rough compression stats
        total_chars = sum(len(chunk) for chunk in encoder.chunks)
        original_size_mb = total_chars / (1024 * 1024)  # Rough estimate
        compression_ratio = original_size_mb / build_stats['video_size_mb'] if build_stats['video_size_mb'] > 0 else 0
        print(f"  📦 Estimated compression ratio: {compression_ratio:.1f}x")

    # Save metadata about this memory
    metadata = {
        'created': datetime.now().isoformat(),
        'source_files': files,
        'video_path': str(video_path),
        'index_path': str(index_path),
        'processing_stats': {
            'files_processed': processed_count,
            'files_skipped': skipped_count,
            'processing_time_seconds': processing_time,
            'encoding_time_seconds': encoding_time,
            'total_time_seconds': total_time
        },
        'build_stats': build_stats
    }

    metadata_path = output_dir / f"{memory_name}_metadata.json"
    with open(metadata_path, 'w') as f:
        json.dump(metadata, f, indent=2)

    print(f"  📄 Metadata: {metadata_path}")

    return str(video_path), str(index_path)

def load_existing_memory(memory_path):
    """Load an existing memory from the output directory"""
    memory_path = Path(memory_path)

    # Handle different input formats
    if memory_path.is_dir():
        # Directory provided, look for memory files
        video_files = list(memory_path.glob(f"*.{VIDEO_FILE_TYPE}"))
        if not video_files:
            raise ValueError(f"No .{VIDEO_FILE_TYPE} files found in {memory_path}")

        video_path = video_files[0]
        index_path = video_path.with_suffix('_index.json')

    elif memory_path.suffix == f'.{VIDEO_FILE_TYPE}':
        # Video file provided
        video_path = memory_path
        index_path = memory_path.with_name(memory_path.stem + '_index.json')

    else:
        # Assume it's a base name, add extensions
        video_path = memory_path.with_suffix(f'.{VIDEO_FILE_TYPE}')
        index_path = memory_path.with_suffix('_index.json')

    # Check if files exist
    if not video_path.exists():
        raise ValueError(f"Video file not found: {video_path}")
    if not index_path.exists():
        raise ValueError(f"Index file not found: {index_path}")

    print(f"Loading existing memory:")
    print(f"  Video: {video_path}")
    print(f"  Index: {index_path}")

    return str(video_path), str(index_path)

def start_chat_session(video_path, index_path, provider='google', model=None):
    """Start an interactive chat session"""
    print(f"\nInitializing chat with {provider}...")

    try:
        chat = MemvidChat(
            video_file=video_path,
            index_file=index_path,
            llm_provider=provider,
            llm_model=model
        )

        print("✓ Chat initialized successfully!")
        print("\nStarting interactive session...")
        print("Commands:")
        print("  - Type your questions normally")
        print("  - Type 'quit' or 'exit' to end")
        print("  - Type 'clear' to clear conversation history")
        print("  - Type 'stats' to see session statistics")
        print("=" * 50)

        # Start interactive chat
        while True:
            try:
                user_input = input("\nYou: ").strip()

                if user_input.lower() in ['quit', 'exit', 'q']:
                    # Export conversation before exiting
                    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
                    export_path = Path("output") / f"conversation_{timestamp}.json"
                    chat.export_conversation(str(export_path))
                    print("Goodbye!")
                    break

                elif user_input.lower() == 'clear':
                    chat.clear_history()
                    continue

                elif user_input.lower() == 'stats':
                    stats = chat.get_stats()
                    print(f"Session stats: {stats}")
                    continue

                if not user_input:
                    continue

                # Get response (always stream for better UX)
                chat.chat(user_input, stream=True)

            except KeyboardInterrupt:
                print("\nGoodbye!")
                break
            except Exception as e:
                print(f"Error: {e}")

    except Exception as e:
        print(f"Error initializing chat: {e}")
        return False

    return True

def main():
    parser = argparse.ArgumentParser(
        description="Chat with your documents using MemVid",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )

    # Input options (mutually exclusive)
    input_group = parser.add_mutually_exclusive_group(required=True)
    input_group.add_argument(
        '--input-dir',
        help='Directory containing documents to process'
    )
    input_group.add_argument(
        '--files',
        nargs='+',
        help='Specific files to process'
    )
    input_group.add_argument(
        '--load-existing',
        help=f'Load existing memory (provide path to {VIDEO_FILE_TYPE} file or directory)'
    )

    # LLM options
    parser.add_argument(
        '--provider',
        choices=['openai', 'google', 'anthropic'],
        default='google',
        help='LLM provider to use (default: google)'
    )
    parser.add_argument(
        '--model',
        help='Specific model to use (uses provider defaults if not specified)'
    )

    # Memory options
    parser.add_argument(
        '--memory-name',
        help='Custom name for the memory files (auto-generated if not provided)'
    )

    # Processing options
    parser.add_argument(
        '--extensions',
        nargs='+',
        default=['.txt', '.md', '.pdf', '.doc', '.docx', '.epub', '.html', '.htm'],
        help='File extensions to include when processing directories'
    )

    args = parser.parse_args()

    # Setup output directory
    output_dir = setup_output_dir()

    try:
        # Get or create memory
        if args.load_existing:
            video_path, index_path = load_existing_memory(args.load_existing)
        else:
            # Collect files
            if args.input_dir:
                files = collect_files_from_directory(args.input_dir, set(args.extensions))
                if not files:
                    print(f"No supported files found in {args.input_dir}")
                    return 1
                input_source = args.input_dir
            else:
                files = args.files
                for f in files:
                    if not Path(f).exists():
                        print(f"File not found: {f}")
                        return 1
                input_source = files

            # Generate memory name
            memory_name = args.memory_name or generate_memory_name(input_source)

            # Create memory
            video_path, index_path = create_memory_from_files(files, output_dir, memory_name)

        # Start chat session
        success = start_chat_session(video_path, index_path, args.provider, args.model)
        return 0 if success else 1

    except Exception as e:
        print(f"Error: {e}")
        return 1

if __name__ == "__main__":
    sys.exit(main())