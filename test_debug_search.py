#!/usr/bin/env python3
"""Debug why search uses metadata instead of decoded QR"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import logging
logging.basicConfig(level=logging.DEBUG)

from memvid import MemvidRetriever
import json

def debug_search():
    print("Debugging search flow...")
    print("=" * 50)
    
    retriever = MemvidRetriever("output/memory.mp4", "output/memory_index.json")
    
    # Patch the decode methods to add logging
    original_decode_parallel = retriever._decode_frames_parallel
    original_decode_single = retriever._decode_single_frame
    
    def logged_decode_parallel(frame_numbers):
        print(f"\n_decode_frames_parallel called with frames: {frame_numbers}")
        result = original_decode_parallel(frame_numbers)
        print(f"Decoded {len(result)} frames: {list(result.keys())}")
        for frame, data in result.items():
            if data:
                print(f"  Frame {frame}: {len(data)} chars decoded")
            else:
                print(f"  Frame {frame}: FAILED to decode")
        return result
    
    def logged_decode_single(frame_number):
        print(f"\n_decode_single_frame called for frame: {frame_number}")
        result = original_decode_single(frame_number)
        if result:
            print(f"  Success: {len(result)} chars")
        else:
            print(f"  FAILED")
        return result
    
    retriever._decode_frames_parallel = logged_decode_parallel
    retriever._decode_single_frame = logged_decode_single
    
    # Do a search
    print("\nPerforming search for 'quantum computing'...")
    results = retriever.search_with_metadata("quantum computing", top_k=3)
    
    print("\n\nFinal results:")
    for i, result in enumerate(results):
        print(f"\n{i+1}. Frame {result['frame']}, Score: {result['score']:.3f}")
        print(f"   Text from result: {result['text'][:50]}...")
        if 'metadata' in result:
            print(f"   Text from metadata: {result['metadata']['text'][:50]}...")
            print(f"   Are they same? {result['text'] == result['metadata']['text']}")

if __name__ == "__main__":
    debug_search()