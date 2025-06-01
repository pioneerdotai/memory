#!/usr/bin/env python3

import os
import sys
import json
import tempfile
import subprocess
from pathlib import Path
from tqdm import tqdm
import time

def create_qr_frame(text, output_path, frame_size=512):
    """Create a QR code frame for the given text."""
    import qrcode
    import cv2
    import numpy as np

    # Create QR code
    qr = qrcode.QRCode(
        version=None,
        error_correction=qrcode.constants.ERROR_CORRECT_H,
        box_size=8,
        border=4,
    )

    qr.add_data(text.encode('utf-8'))
    qr.make(fit=True)

    # Convert to image
    qr_img = qr.make_image(fill_color="black", back_color="white")
    qr_array = np.array(qr_img.convert('RGB'))

    # Resize to target size
    qr_resized = cv2.resize(qr_array, (frame_size, frame_size),
                            interpolation=cv2.INTER_NEAREST)

    # Save frame
    cv2.imwrite(str(output_path), qr_resized)

def encode_to_h265(frames_dir, output_path, fps=30, crf=23):
    """Encode frames to H.265 video using FFmpeg."""

    cmd = [
        'ffmpeg', '-y',
        '-framerate', str(fps),
        '-i', str(frames_dir / 'frame_%06d.png'),
        '-c:v', 'libx265',
        '-preset', 'medium',  # Good balance of speed vs compression
        '-crf', str(crf),
        '-pix_fmt', 'yuv420p',
        output_path
    ]

    print("🎬 Running FFmpeg encoding...")
    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode != 0:
        raise RuntimeError(f"FFmpeg failed: {result.stderr}")

    print("✅ H.265 encoding complete")

def main():
    if len(sys.argv) != 3:
        print("Usage: python dockerized_encoder.py INPUT_FILE OUTPUT_FILE")
        print("  INPUT_FILE: JSON file with list of text chunks")
        print("  OUTPUT_FILE: Output H.265 video file")
        sys.exit(1)

    input_file = sys.argv[1]
    output_file = sys.argv[2]

    # Handle container paths
    if not os.path.isabs(input_file):
        input_file = f"/data/input/{input_file}"
    if not os.path.isabs(output_file):
        output_file = f"/data/output/{output_file}"

    print(f"📖 Reading chunks from: {input_file}")
    print(f"💾 Will save video to: {output_file}")

    # Load input chunks
    try:
        with open(input_file, 'r', encoding='utf-8') as f:
            chunks = json.load(f)
    except Exception as e:
        print(f"❌ Error reading input file: {e}")
        sys.exit(1)

    if not isinstance(chunks, list):
        print("❌ Input file must contain a JSON list of strings")
        sys.exit(1)

    print(f"📊 Loaded {len(chunks)} chunks")

    # Create temporary directory for frames
    with tempfile.TemporaryDirectory(dir="/data/temp") as temp_dir:
        frames_dir = Path(temp_dir) / "frames"
        frames_dir.mkdir()

        print("🎨 Generating QR code frames...")

        # Generate QR frames with progress bar
        for i, chunk in enumerate(tqdm(chunks, desc="Creating frames")):
            frame_path = frames_dir / f"frame_{i:06d}.png"
            create_qr_frame(chunk, frame_path)

        print("✅ All frames generated")

        # Encode to H.265
        start_time = time.time()
        encode_to_h265(frames_dir, output_file)
        encoding_time = time.time() - start_time

        # Create index file
        index_path = Path(output_file).with_suffix('.json')
        index_data = {
            'version': '1.0',
            'chunks': chunks,
            'encoding': {
                'codec': 'h265',
                'fps': 30,
                'crf': 23,
                'total_chunks': len(chunks),
                'encoding_time_seconds': round(encoding_time, 2)
            }
        }

        with open(index_path, 'w', encoding='utf-8') as f:
            json.dump(index_data, f, indent=2, ensure_ascii=False)

        # Show stats
        file_size = Path(output_file).stat().st_size
        file_size_mb = file_size / (1024 * 1024)
        chunks_per_mb = len(chunks) / file_size_mb

        print(f"🎉 Encoding complete!")
        print(f"   Video: {output_file}")
        print(f"   Index: {index_path}")
        print(f"   Size: {file_size_mb:.2f} MB")
        print(f"   Density: {chunks_per_mb:.0f} chunks/MB")
        print(f"   Time: {encoding_time:.1f} seconds")

if __name__ == '__main__':
    main()