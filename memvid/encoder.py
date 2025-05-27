"""
MemvidEncoder - Handles chunking and QR video creation
"""

import json
import logging
from pathlib import Path
from typing import List, Optional, Dict, Any
from tqdm import tqdm
import cv2
import numpy as np

from .utils import encode_to_qr, qr_to_frame, create_video_writer, chunk_text
from .index import IndexManager
from .config import get_default_config

logger = logging.getLogger(__name__)


class MemvidEncoder:
    """Encodes text chunks into QR code videos with searchable index"""
    
    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """
        Initialize MemvidEncoder
        
        Args:
            config: Optional configuration dictionary
        """
        self.config = config or get_default_config()
        self.chunks = []
        self.index_manager = IndexManager(self.config)
        
    def add_chunks(self, chunks: List[str]):
        """
        Add text chunks to be encoded
        
        Args:
            chunks: List of text chunks
        """
        self.chunks.extend(chunks)
        logger.info(f"Added {len(chunks)} chunks. Total: {len(self.chunks)}")
    
    def add_text(self, text: str, chunk_size: int = 500, overlap: int = 50):
        """
        Add text and automatically chunk it
        
        Args:
            text: Text to chunk and add
            chunk_size: Target chunk size
            overlap: Overlap between chunks
        """
        chunks = chunk_text(text, chunk_size, overlap)
        self.add_chunks(chunks)
    
    def build_video(self, output_file: str, index_file: str, 
                    show_progress: bool = True) -> Dict[str, Any]:
        """
        Build QR code video and index from chunks
        
        Args:
            output_file: Path to output video file
            index_file: Path to output index file
            show_progress: Show progress bar
            
        Returns:
            Dictionary with build statistics
        """
        if not self.chunks:
            raise ValueError("No chunks to encode. Use add_chunks() first.")
        
        output_path = Path(output_file)
        index_path = Path(index_file)
        
        # Ensure output directory exists
        output_path.parent.mkdir(parents=True, exist_ok=True)
        index_path.parent.mkdir(parents=True, exist_ok=True)
        
        logger.info(f"Building video with {len(self.chunks)} chunks")
        
        # Create video writer
        video_config = self.config["video"]
        writer = create_video_writer(str(output_path), video_config)
        
        frame_numbers = []
        
        try:
            # Generate QR codes and write to video
            chunks_iter = enumerate(self.chunks)
            if show_progress:
                chunks_iter = tqdm(chunks_iter, total=len(self.chunks), desc="Encoding chunks to video")
            
            for frame_num, chunk in chunks_iter:
                # Create metadata for chunk
                chunk_data = {
                    "id": frame_num,
                    "text": chunk,
                    "frame": frame_num
                }
                
                # Encode to QR
                qr_image = encode_to_qr(json.dumps(chunk_data), self.config)
                
                # Convert to video frame
                frame = qr_to_frame(qr_image, (video_config["frame_width"], video_config["frame_height"]))
                
                # Write frame
                writer.write(frame)
                frame_numbers.append(frame_num)
            
            # Add chunks to index
            logger.info("Building search index...")
            self.index_manager.add_chunks(self.chunks, frame_numbers, show_progress)
            
            # Save index
            self.index_manager.save(str(index_path.with_suffix('')))
            
            # Get statistics
            stats = {
                "total_chunks": len(self.chunks),
                "total_frames": len(frame_numbers),
                "video_file": str(output_path),
                "index_file": str(index_path),
                "video_size_mb": output_path.stat().st_size / (1024 * 1024) if output_path.exists() else 0,
                "fps": video_config["fps"],
                "duration_seconds": len(frame_numbers) / video_config["fps"],
                "index_stats": self.index_manager.get_stats()
            }
            
            logger.info(f"Successfully built video: {output_path}")
            logger.info(f"Video duration: {stats['duration_seconds']:.1f} seconds")
            logger.info(f"Video size: {stats['video_size_mb']:.1f} MB")
            
            return stats
            
        finally:
            writer.release()
    
    def clear(self):
        """Clear all chunks"""
        self.chunks = []
        self.index_manager = IndexManager(self.config)
        logger.info("Cleared all chunks")
    
    def get_stats(self) -> Dict[str, Any]:
        """Get encoder statistics"""
        return {
            "total_chunks": len(self.chunks),
            "total_characters": sum(len(chunk) for chunk in self.chunks),
            "avg_chunk_size": np.mean([len(chunk) for chunk in self.chunks]) if self.chunks else 0,
            "config": self.config
        }
    
    @classmethod
    def from_file(cls, file_path: str, chunk_size: int = 500, 
                  overlap: int = 50, config: Optional[Dict[str, Any]] = None) -> 'MemvidEncoder':
        """
        Create encoder from text file
        
        Args:
            file_path: Path to text file
            chunk_size: Target chunk size
            overlap: Overlap between chunks
            config: Optional configuration
            
        Returns:
            MemvidEncoder instance with chunks loaded
        """
        encoder = cls(config)
        
        with open(file_path, 'r', encoding='utf-8') as f:
            text = f.read()
        
        encoder.add_text(text, chunk_size, overlap)
        return encoder
    
    @classmethod
    def from_documents(cls, documents: List[str], chunk_size: int = 500,
                      overlap: int = 50, config: Optional[Dict[str, Any]] = None) -> 'MemvidEncoder':
        """
        Create encoder from list of documents
        
        Args:
            documents: List of document strings
            chunk_size: Target chunk size
            overlap: Overlap between chunks
            config: Optional configuration
            
        Returns:
            MemvidEncoder instance with chunks loaded
        """
        encoder = cls(config)
        
        for doc in documents:
            encoder.add_text(doc, chunk_size, overlap)
        
        return encoder