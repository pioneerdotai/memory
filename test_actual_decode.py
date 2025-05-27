#!/usr/bin/env python3
"""Test if QR decoding from MP4 actually works"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import cv2
from memvid.utils import decode_qr, extract_frame
import json

def test_direct_decode():
    video_file = "output/memory.mp4"
    
    print("Testing direct QR decode from MP4...")
    print("=" * 50)
    
    # Try to extract and decode frame 0
    print("\nExtracting frame 0...")
    frame = extract_frame(video_file, 0)
    if frame is not None:
        print(f"Frame extracted successfully: shape={frame.shape}")
        
        # Try to decode QR
        print("\nDecoding QR code...")
        decoded = decode_qr(frame)
        if decoded:
            print("QR decode successful!")
            try:
                data = json.loads(decoded)
                print(f"Decoded text: {data.get('text', 'NO TEXT')[:100]}...")
            except:
                print(f"Raw decoded data: {decoded[:100]}...")
        else:
            print("QR decode FAILED - decode_qr returned None/empty")
            
            # Save frame for inspection
            cv2.imwrite("test_frame_0.png", frame)
            print("Saved frame to test_frame_0.png for inspection")
    else:
        print("Frame extraction FAILED")
    
    # Also test the retriever's internal decode method
    print("\n\nTesting retriever's decode method...")
    from memvid import MemvidRetriever
    retriever = MemvidRetriever(video_file, "output/memory_index.json")
    
    # Try to decode a single frame
    decoded = retriever._decode_single_frame(0)
    if decoded:
        print("Retriever decode successful!")
        print(f"Decoded: {decoded[:100]}...")
    else:
        print("Retriever decode FAILED")
    
    # Check the decode parallel method
    print("\n\nTesting parallel decode...")
    decoded_frames = retriever._decode_frames_parallel([0, 1, 2])
    print(f"Decoded frames: {list(decoded_frames.keys())}")
    for frame_num, data in decoded_frames.items():
        if data:
            print(f"Frame {frame_num}: {data[:50]}...")
        else:
            print(f"Frame {frame_num}: FAILED")

if __name__ == "__main__":
    test_direct_decode()