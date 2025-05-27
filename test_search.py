#!/usr/bin/env python3
"""Test search functionality directly"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# Set environment variable to avoid tokenizers warning
os.environ['TOKENIZERS_PARALLELISM'] = 'false'

from memvid import MemvidRetriever

def main():
    # Initialize retriever
    retriever = MemvidRetriever("output/memory.mp4", "output/memory_index.json")
    
    # Get stats
    stats = retriever.get_stats()
    print(f"Retriever stats: {stats}")
    
    # Test search
    query = "what's this context about"
    print(f"\nSearching for: '{query}'")
    
    # Search with metadata to see what's being found
    results_with_metadata = retriever.search_with_metadata(query, top_k=5)
    
    print(f"\nFound {len(results_with_metadata)} results:")
    for i, result in enumerate(results_with_metadata):
        print(f"\n[{i+1}] Score: {result['score']:.3f}, Frame: {result['frame']}")
        print(f"Text preview: {result['text'][:100]}...")
    
    # Also test direct search
    results = retriever.search(query, top_k=5)
    print(f"\n\nDirect search returned {len(results)} results")
    for i, text in enumerate(results):
        print(f"\n[{i+1}] {text[:100]}...")

if __name__ == "__main__":
    main()