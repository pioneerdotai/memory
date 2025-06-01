#!/usr/bin/env python3
# Save this as: docker/scripts/test_encoding.py

import json
import tempfile
import subprocess
from pathlib import Path

def test_encoding():
    """Run a simple test to verify H.265 encoding works."""

    print("🧪 Testing H.265 encoding capability...")

    # Create test data
    test_chunks = [
        "Hello, this is test chunk 1",
        "This is test chunk 2 with more content",
        "Final test chunk with special chars: áéíóú"
    ]

    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)

        # Save test input
        input_file = temp_path / "test_input.json"
        with open(input_file, 'w') as f:
            json.dump(test_chunks, f)

        output_file = temp_path / "test_output.mp4"

        # Test encoding
        try:
            cmd = ['python3', '/scripts/h265_encode_optimized.py',
                   str(input_file), str(output_file)]

            result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)

            if result.returncode == 0 and output_file.exists():
                file_size = output_file.stat().st_size
                print(f"✅ Test passed!")
                print(f"   Created {file_size} byte H.265 video")
                print(f"   Encoded {len(test_chunks)} test chunks")
                return True
            else:
                print(f"❌ Test failed: {result.stderr}")
                return False

        except subprocess.TimeoutExpired:
            print("❌ Test timed out")
            return False
        except Exception as e:
            print(f"❌ Test error: {e}")
            return False

if __name__ == '__main__':
    success = test_encoding()
    exit(0 if success else 1)