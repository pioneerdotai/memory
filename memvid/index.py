"""
Index management for embeddings and vector search
"""

import json
import numpy as np
import faiss
from sentence_transformers import SentenceTransformer
from typing import List, Dict, Any, Tuple, Optional
import logging
from pathlib import Path
import pickle
from tqdm import tqdm

from .config import get_default_config

logger = logging.getLogger(__name__)


class IndexManager:
    """Manages embeddings, FAISS index, and metadata for fast retrieval"""
    
    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """
        Initialize IndexManager
        
        Args:
            config: Optional configuration dictionary
        """
        self.config = config or get_default_config()
        self.embedding_model = SentenceTransformer(self.config["embedding"]["model"])
        self.dimension = self.config["embedding"]["dimension"]
        
        # Initialize FAISS index
        self.index = self._create_index()
        
        # Metadata storage
        self.metadata = []
        self.chunk_to_frame = {}  # Maps chunk ID to frame number
        self.frame_to_chunks = {}  # Maps frame number to chunk IDs
        
    def _create_index(self) -> faiss.Index:
        """Create FAISS index based on configuration"""
        index_type = self.config["index"]["type"]
        
        if index_type == "Flat":
            # Exact search - best quality, slower for large datasets
            index = faiss.IndexFlatL2(self.dimension)
        elif index_type == "IVF":
            # Inverted file index - faster for large datasets
            quantizer = faiss.IndexFlatL2(self.dimension)
            index = faiss.IndexIVFFlat(quantizer, self.dimension, self.config["index"]["nlist"])
        else:
            raise ValueError(f"Unknown index type: {index_type}")
            
        # Add ID mapping for retrieval
        index = faiss.IndexIDMap(index)
        return index
    
    def add_chunks(self, chunks: List[str], frame_numbers: List[int], 
                   show_progress: bool = True) -> List[int]:
        """
        Add chunks to index with their frame mappings
        
        Args:
            chunks: List of text chunks
            frame_numbers: Corresponding frame numbers for each chunk
            show_progress: Show progress bar
            
        Returns:
            List of chunk IDs
        """
        if len(chunks) != len(frame_numbers):
            raise ValueError("Number of chunks must match number of frame numbers")
        
        # Generate embeddings
        logger.info(f"Generating embeddings for {len(chunks)} chunks")
        
        # SentenceTransformer expects a list, not an iterator
        embeddings = self.embedding_model.encode(
            chunks, 
            show_progress_bar=show_progress,
            batch_size=32
        )
        embeddings = np.array(embeddings).astype('float32')
        
        # Assign IDs
        start_id = len(self.metadata)
        chunk_ids = list(range(start_id, start_id + len(chunks)))
        
        # Train index if needed (for IVF)
        if isinstance(self.index.index, faiss.IndexIVFFlat) and not self.index.index.is_trained:
            logger.info("Training FAISS index...")
            self.index.index.train(embeddings)
        
        # Add to index
        self.index.add_with_ids(embeddings, np.array(chunk_ids, dtype=np.int64))
        
        # Store metadata
        for i, (chunk, frame_num, chunk_id) in enumerate(zip(chunks, frame_numbers, chunk_ids)):
            metadata = {
                "id": chunk_id,
                "text": chunk,
                "frame": frame_num,
                "length": len(chunk)
            }
            self.metadata.append(metadata)
            
            # Update mappings
            self.chunk_to_frame[chunk_id] = frame_num
            if frame_num not in self.frame_to_chunks:
                self.frame_to_chunks[frame_num] = []
            self.frame_to_chunks[frame_num].append(chunk_id)
        
        logger.info(f"Added {len(chunks)} chunks to index")
        return chunk_ids
    
    def search(self, query: str, top_k: int = 5) -> List[Tuple[int, float, Dict[str, Any]]]:
        """
        Search for similar chunks
        
        Args:
            query: Search query
            top_k: Number of results to return
            
        Returns:
            List of (chunk_id, distance, metadata) tuples
        """
        # Generate query embedding
        query_embedding = self.embedding_model.encode([query])
        query_embedding = np.array(query_embedding).astype('float32')
        
        # Search
        distances, indices = self.index.search(query_embedding, top_k)
        
        # Gather results
        results = []
        for dist, idx in zip(distances[0], indices[0]):
            if idx >= 0:  # Valid result
                metadata = self.metadata[idx]
                results.append((idx, float(dist), metadata))
        
        return results
    
    def get_chunks_by_frame(self, frame_number: int) -> List[Dict[str, Any]]:
        """Get all chunks associated with a frame"""
        chunk_ids = self.frame_to_chunks.get(frame_number, [])
        return [self.metadata[chunk_id] for chunk_id in chunk_ids]
    
    def get_chunk_by_id(self, chunk_id: int) -> Optional[Dict[str, Any]]:
        """Get chunk metadata by ID"""
        if 0 <= chunk_id < len(self.metadata):
            return self.metadata[chunk_id]
        return None
    
    def save(self, path: str):
        """
        Save index to disk
        
        Args:
            path: Path to save index (without extension)
        """
        path = Path(path)
        
        # Save FAISS index
        faiss.write_index(self.index, str(path.with_suffix('.faiss')))
        
        # Save metadata and mappings
        data = {
            "metadata": self.metadata,
            "chunk_to_frame": self.chunk_to_frame,
            "frame_to_chunks": self.frame_to_chunks,
            "config": self.config
        }
        
        with open(path.with_suffix('.json'), 'w') as f:
            json.dump(data, f, indent=2)
        
        logger.info(f"Saved index to {path}")
    
    def load(self, path: str):
        """
        Load index from disk
        
        Args:
            path: Path to load index from (without extension)
        """
        path = Path(path)
        
        # Load FAISS index
        self.index = faiss.read_index(str(path.with_suffix('.faiss')))
        
        # Load metadata and mappings
        with open(path.with_suffix('.json'), 'r') as f:
            data = json.load(f)
        
        self.metadata = data["metadata"]
        self.chunk_to_frame = {int(k): v for k, v in data["chunk_to_frame"].items()}
        self.frame_to_chunks = {int(k): v for k, v in data["frame_to_chunks"].items()}
        
        # Update config if available
        if "config" in data:
            self.config.update(data["config"])
        
        logger.info(f"Loaded index from {path}")
    
    def get_stats(self) -> Dict[str, Any]:
        """Get index statistics"""
        return {
            "total_chunks": len(self.metadata),
            "total_frames": len(self.frame_to_chunks),
            "index_type": self.config["index"]["type"],
            "embedding_model": self.config["embedding"]["model"],
            "dimension": self.dimension,
            "avg_chunks_per_frame": np.mean([len(chunks) for chunks in self.frame_to_chunks.values()]) if self.frame_to_chunks else 0
        }