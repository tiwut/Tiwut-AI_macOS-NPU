import json
import re
import time
from pathlib import Path
from typing import List, Dict, Tuple, Optional
import torch
import torch.nn.functional as F

from config import HardwareManager, PathConfig
from database import HighSpeedNeuralDatabase
from model import TiwutNeuralAI
from tokenizer import ByteLevelTokenizer


class RAMNeuralMemoryBank:

    def __init__(
        self,
        db: HighSpeedNeuralDatabase,
        device: torch.device,
        paths: PathConfig
    ):
        self.db = db
        self.device = device
        self.paths = paths
        self.chunks: List[Dict] = []
        self.semantic_matrix: Optional[torch.Tensor] = None
        self.metadata: Dict = {
            "total_documents": 0,
            "total_chunks": 0,
            "total_tokens": 0,
            "sources": [],
            "last_updated": 0,
            "ram_size_mb": 0.0
        }
        self.load_from_database()

    def add_document_and_chunks(
        self,
        source: str,
        title: str,
        raw_text: str,
        chunks: List[Dict],
        model: TiwutNeuralAI,
        batch_size: int = 32
    ):
        if not chunks:
            return

        model.eval()
        new_vectors = []

        with torch.no_grad():
            for i in range(0, len(chunks), batch_size):
                batch_chunks = chunks[i:i + batch_size]
                batch_tokens = [c["tokens"] for c in batch_chunks]
                max_len = max(len(t) for t in batch_tokens)
                padded_tokens = [t + [0] * (max_len - len(t)) for t in batch_tokens]
                input_tensor = torch.tensor(padded_tokens, dtype=torch.long, device=self.device)

                try:
                    vectors = model.encode_semantic_vector(input_tensor)
                except Exception:
                    vectors = torch.zeros((len(batch_chunks), model.config.embed_dim), device=self.device)
                new_vectors.append(vectors)

        new_matrix = torch.cat(new_vectors, dim=0)

        self.db.save_document_and_chunks(
            source=source,
            title=title,
            raw_text=raw_text,
            chunks=chunks,
            embeddings=new_matrix
        )

        if self.semantic_matrix is None or len(self.chunks) == 0:
            self.semantic_matrix = new_matrix
            self.chunks = list(chunks)
        else:
            self.semantic_matrix = torch.cat([self.semantic_matrix, new_matrix], dim=0)
            self.chunks.extend(chunks)

        self._refresh_metadata()

    def _refresh_metadata(self):
        db_stats = self.db.get_stats()
        self.metadata["total_documents"] = db_stats["total_documents"]
        self.metadata["total_chunks"] = db_stats["total_chunks"]
        self.metadata["total_tokens"] = db_stats["total_tokens"]
        self.metadata["sources"] = db_stats["sources"]
        self.metadata["last_updated"] = time.time()
        self.metadata["db_size_mb"] = db_stats["database_size_mb"]

        ram_bytes = 0
        if self.semantic_matrix is not None:
            ram_bytes += self.semantic_matrix.element_size() * self.semantic_matrix.nelement()
        for c in self.chunks:
            ram_bytes += len(c.get("text", "").encode("utf-8"))
            ram_bytes += len(c.get("tokens", [])) * 8

        self.metadata["ram_size_mb"] = round(ram_bytes / (1024 * 1024), 3)

    @torch.no_grad()
    def search_neural_memory(
        self,
        query: str,
        model: TiwutNeuralAI,
        tokenizer: ByteLevelTokenizer,
        top_k: int = 3,
        threshold: float = 0.1
    ) -> List[Tuple[Dict, float]]:
        if self.semantic_matrix is None or len(self.chunks) == 0:
            return []

        model.eval()
        q_tokens = tokenizer.encode(query, add_special_tokens=True)
        q_tensor = torch.tensor([q_tokens], dtype=torch.long, device=self.device)

        try:
            q_vector = model.encode_semantic_vector(q_tensor)
            sim_scores = torch.matmul(self.semantic_matrix, q_vector.T).squeeze(-1)
            if sim_scores.numel() > 1:
                centered_sim = (sim_scores - sim_scores.mean()) / (sim_scores.std() + 1e-6)
                norm_sim = torch.sigmoid(centered_sim)
            else:
                norm_sim = sim_scores
        except Exception:
            norm_sim = torch.zeros(len(self.chunks), device=self.device)

        STOPWORDS = {
            "what", "is", "and", "the", "a", "an", "of", "to", "in", "on", "for", "with",
            "does", "do", "it", "its", "have", "has", "this", "that", "how", "why", "who",
            "where", "when", "can", "could", "be", "are", "was", "were", "been", "by", "at",
            "which", "there", "their", "they", "from", "about"
        }
        raw_words = re.findall(r"[A-Za-z0-9_\-]+", query.lower())
        q_words = [w for w in raw_words if w not in STOPWORDS and len(w) > 1]
        lexical_scores = torch.zeros_like(norm_sim)

        if q_words:
            for idx, chunk in enumerate(self.chunks):
                chunk_text_lower = chunk.get("text", "").lower()
                matches = 0
                for w in q_words:
                    if w in chunk_text_lower:
                        matches += 1
                if matches > 0:
                    lexical_scores[idx] = float(matches) / len(q_words)

        if q_words and lexical_scores.max() > 0:
            final_scores = 0.40 * norm_sim + 0.60 * lexical_scores
        else:
            final_scores = norm_sim

        top_k_val = min(top_k, len(self.chunks))
        top_scores, top_indices = torch.topk(final_scores, top_k_val)

        results = []
        for score, idx in zip(top_scores.tolist(), top_indices.tolist()):
            if score >= threshold or (q_words and lexical_scores[idx] > 0):
                results.append((self.chunks[idx], float(score)))

        return results

    def extract_intelligent_answer(self, query: str, top_chunks: List[Tuple[Dict, float]]) -> Optional[str]:
        if not top_chunks:
            return None

        STOPWORDS = {
            "what", "is", "and", "the", "a", "an", "of", "to", "in", "on", "for", "with",
            "does", "do", "it", "its", "have", "has", "this", "that", "how", "why", "who",
            "where", "when", "can", "could", "be", "are", "was", "were", "been", "by", "at"
        }
        raw_query_words = re.findall(r"[A-Za-z0-9_\-]+", query.lower())
        q_words = [w for w in raw_query_words if w not in STOPWORDS and len(w) > 1]

        best_sentences = []
        seen_sentences = set()

        for chunk_meta, chunk_score in top_chunks:
            text = chunk_meta.get("text", "")
            raw_lines = text.split("\n")
            for line in raw_lines:
                clean_line = line.strip()
                if not clean_line or clean_line.startswith(("Source:", "Title:", "Document:")):
                    continue

                line_sentences = re.split(r"(?<=[.!?])\s+", clean_line)
                for s in line_sentences:
                    s_clean = s.strip()
                    if len(s_clean) < 15 or s_clean in seen_sentences:
                        continue

                    s_lower = s_clean.lower()
                    score = 0
                    if q_words:
                        for w in q_words:
                            if w in s_lower:
                                score += 1

                    seen_sentences.add(s_clean)
                    best_sentences.append((s_clean, score))

        if not best_sentences:
            top_raw = top_chunks[0][0].get("text", "").strip()
            clean_lines = [l.strip() for l in top_raw.split("\n") if l.strip() and not l.startswith(("Source:", "Title:", "Document:"))]
            return "\n".join(clean_lines[:6]) if clean_lines else top_raw

        best_sentences.sort(key=lambda x: x[1], reverse=True)
        top_selected = [s[0] for s in best_sentences[:5] if s[1] > 0 or len(best_sentences) <= 3]
        if not top_selected:
            top_selected = [s[0] for s in best_sentences[:4]]

        return "\n".join(top_selected)

    def load_from_database(self):
        try:
            chunks, embeddings = self.db.load_all_chunks_and_embeddings(self.device)
            self.chunks = chunks
            self.semantic_matrix = embeddings
            self._refresh_metadata()
        except Exception:
            pass

    def clear(self):
        self.semantic_matrix = None
        self.chunks = []
        self.db.clear_all()
        self._refresh_metadata()
