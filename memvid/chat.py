"""
MemvidChat - Conversational interface with context-aware memory
"""

import os
import logging
from typing import List, Dict, Any, Optional, Tuple
from datetime import datetime
import json

from .retriever import MemvidRetriever
from .config import get_default_config

logger = logging.getLogger(__name__)


try:
    from openai import OpenAI
    OPENAI_AVAILABLE = True
except ImportError:
    OPENAI_AVAILABLE = False
    logger.warning("OpenAI library not available. LLM features will be limited.")


class MemvidChat:
    """Manages conversations with context retrieval and LLM interface"""
    
    def __init__(self, video_file: str, index_file: str,
                 llm_api_key: Optional[str] = None,
                 llm_model: Optional[str] = None,
                 config: Optional[Dict[str, Any]] = None):
        """
        Initialize MemvidChat
        
        Args:
            video_file: Path to QR code video
            index_file: Path to index file
            llm_api_key: API key for LLM (or set via environment)
            llm_model: LLM model to use
            config: Optional configuration
        """
        self.config = config or get_default_config()
        self.retriever = MemvidRetriever(video_file, index_file, self.config)
        
        # Initialize LLM
        self.llm_model = llm_model or self.config["llm"]["model"]
        self._init_llm(llm_api_key)
        
        # Conversation state
        self.conversation_history = []
        self.session_id = None
        self.context_chunks = self.config["chat"]["context_chunks"]
        self.max_history = self.config["chat"]["max_history"]
        
    def _init_llm(self, api_key: Optional[str] = None):
        """Initialize LLM client"""
        if not OPENAI_AVAILABLE:
            self.llm_client = None
            logger.warning("OpenAI not available. Chat will return context only.")
            return
        
        # Get API key from parameter, env, or config
        api_key = api_key or os.getenv("OPENAI_API_KEY")
        if not api_key:
            self.llm_client = None
            logger.warning("No OpenAI API key provided. Chat will return context only.")
            return
        
        try:
            self.llm_client = OpenAI(api_key=api_key)
            logger.info(f"Initialized OpenAI client with model: {self.llm_model}")
        except Exception as e:
            self.llm_client = None
            logger.error(f"Failed to initialize OpenAI client: {e}")
    
    def start_session(self, session_id: Optional[str] = None):
        """
        Start a new chat session
        
        Args:
            session_id: Optional session ID (generates one if not provided)
        """
        self.session_id = session_id or f"session_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
        self.conversation_history = []
        logger.info(f"Started chat session: {self.session_id}")
    
    def chat(self, user_input: str) -> str:
        """
        Process user input and generate response
        
        Args:
            user_input: User's message
            
        Returns:
            Assistant's response
        """
        if not self.session_id:
            self.start_session()
        
        # Retrieve relevant context
        context_chunks = self.retriever.search(user_input, top_k=self.context_chunks)
        
        # Build context string
        context = "\n\n".join([f"[Context {i+1}]: {chunk}" 
                               for i, chunk in enumerate(context_chunks)])
        
        # Add to history
        self.conversation_history.append({
            "role": "user",
            "content": user_input,
            "timestamp": datetime.now().isoformat()
        })
        
        # Generate response
        if self.llm_client:
            response = self._generate_llm_response(user_input, context)
        else:
            # Fallback: return context only
            response = self._generate_context_response(context_chunks)
        
        # Add response to history
        self.conversation_history.append({
            "role": "assistant",
            "content": response,
            "timestamp": datetime.now().isoformat(),
            "context_used": len(context_chunks)
        })
        
        # Trim history if needed
        if len(self.conversation_history) > self.max_history * 2:
            self.conversation_history = self.conversation_history[-self.max_history * 2:]
        
        return response
    
    def _generate_llm_response(self, user_input: str, context: str) -> str:
        """Generate response using LLM"""
        try:
            # Build messages
            messages = [
                {
                    "role": "system",
                    "content": (
                        "You are a helpful assistant with access to a knowledge base stored in video memory. "
                        "Use the provided context chunks to answer questions accurately. "
                        "If asked about what the context or knowledge base contains, analyze and summarize "
                        "the topics covered based on the context chunks provided. "
                        "Always base your answers on the given context."
                    )
                }
            ]
            
            # Add conversation history (last N messages)
            history_start = max(0, len(self.conversation_history) - self.max_history)
            for msg in self.conversation_history[history_start:-1]:  # Exclude current user message
                messages.append({
                    "role": msg["role"],
                    "content": msg["content"]
                })
            
            # Add current query with context
            messages.append({
                "role": "user",
                "content": f"Context from knowledge base:\n{context}\n\nUser question: {user_input}"
            })
            
            # Generate response
            response = self.llm_client.chat.completions.create(
                model=self.llm_model,
                messages=messages,
                max_tokens=self.config["llm"]["max_tokens"],
                temperature=self.config["llm"]["temperature"]
            )
            
            return response.choices[0].message.content
            
        except Exception as e:
            logger.error(f"LLM generation failed: {e}")
            return self._generate_context_response(context.split("\n\n"))
    
    def _generate_context_response(self, context_chunks: List[str]) -> str:
        """Generate response without LLM (context only)"""
        if not context_chunks:
            return "I couldn't find any relevant information in the knowledge base."
        
        # Check if the chunks are actually relevant (not just random matches)
        # If all chunks are very short or seem unrelated, indicate low relevance
        avg_chunk_length = sum(len(chunk) for chunk in context_chunks) / len(context_chunks)
        if avg_chunk_length < 50:  # Likely fragment matches
            return "I couldn't find any relevant information about that topic in the knowledge base."
        
        response = "Based on the knowledge base, here's what I found:\n\n"
        for i, chunk in enumerate(context_chunks[:3]):  # Limit to top 3
            response += f"{i+1}. {chunk[:200]}...\n\n" if len(chunk) > 200 else f"{i+1}. {chunk}\n\n"
        
        return response.strip()
    
    def search_context(self, query: str, top_k: int = 5) -> List[Dict[str, Any]]:
        """
        Search for context without generating a response
        
        Args:
            query: Search query
            top_k: Number of results
            
        Returns:
            List of search results with metadata
        """
        return self.retriever.search_with_metadata(query, top_k)
    
    def get_history(self) -> List[Dict[str, Any]]:
        """Get conversation history"""
        return self.conversation_history.copy()
    
    def export_session(self, output_file: str):
        """
        Export session to file
        
        Args:
            output_file: Path to output file
        """
        session_data = {
            "session_id": self.session_id,
            "start_time": self.conversation_history[0]["timestamp"] if self.conversation_history else None,
            "end_time": self.conversation_history[-1]["timestamp"] if self.conversation_history else None,
            "message_count": len(self.conversation_history),
            "history": self.conversation_history,
            "config": {
                "llm_model": self.llm_model,
                "context_chunks": self.context_chunks,
                "max_history": self.max_history
            }
        }
        
        with open(output_file, 'w') as f:
            json.dump(session_data, f, indent=2)
        
        logger.info(f"Exported session to: {output_file}")
    
    def load_session(self, session_file: str):
        """
        Load session from file
        
        Args:
            session_file: Path to session file
        """
        with open(session_file, 'r') as f:
            session_data = json.load(f)
        
        self.session_id = session_data["session_id"]
        self.conversation_history = session_data["history"]
        
        logger.info(f"Loaded session: {self.session_id}")
    
    def reset_session(self):
        """Reset conversation history"""
        self.conversation_history = []
        logger.info("Reset conversation history")
    
    def get_stats(self) -> Dict[str, Any]:
        """Get chat statistics"""
        return {
            "session_id": self.session_id,
            "message_count": len(self.conversation_history),
            "llm_available": self.llm_client is not None,
            "llm_model": self.llm_model,
            "context_chunks_per_query": self.context_chunks,
            "retriever_stats": self.retriever.get_stats()
        }