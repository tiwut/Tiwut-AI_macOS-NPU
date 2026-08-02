import os
import json
import time
import sqlite3
from pathlib import Path
from typing import List, Dict, Tuple, Optional, Union
import numpy as np
import torch


class HighSpeedNeuralDatabase:

    def __init__(self, db_path: Union[str, Path], mmap_size_mb: int = 256):
        if str(db_path) == ":memory:":
            self.is_memory_db = True
            self.db_path = Path(":memory:")
        else:
            self.is_memory_db = False
            self.db_path = Path(db_path)
            self.db_path.parent.mkdir(parents=True, exist_ok=True)
            
        self.mmap_size_bytes = max(0, mmap_size_mb) * 1024 * 1024
        self._shared_mem_conn: Optional[sqlite3.Connection] = None
        
        if self.is_memory_db:
            self._shared_mem_conn = self._create_connection()
            
        self._init_db()

    def _create_connection(self) -> sqlite3.Connection:
        target = ":memory:" if self.is_memory_db else str(self.db_path)
        conn = sqlite3.connect(target, timeout=60.0, check_same_thread=False)
        conn.row_factory = sqlite3.Row
        
        if not self.is_memory_db:
            try:
                conn.execute("PRAGMA journal_mode = WAL;")
                conn.execute("PRAGMA synchronous = NORMAL;")
                conn.execute(f"PRAGMA mmap_size = {self.mmap_size_bytes};")
                conn.execute("PRAGMA cache_size = -64000;")
                conn.execute("PRAGMA wal_autocheckpoint = 1000;")
            except sqlite3.Error:
                pass

        try:
            conn.execute("PRAGMA temp_store = MEMORY;")
            conn.execute("PRAGMA foreign_keys = ON;")
            conn.execute("PRAGMA busy_timeout = 30000;")
        except sqlite3.Error:
            pass

        return conn

    def _get_connection(self) -> sqlite3.Connection:
        if self.is_memory_db and self._shared_mem_conn is not None:
            return self._shared_mem_conn
        return self._create_connection()

    def _init_db(self):
        conn = self._get_connection()
        try:
            with conn:
                conn.executescript("""
                    CREATE TABLE IF NOT EXISTS documents (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        source TEXT UNIQUE NOT NULL,
                        title TEXT NOT NULL,
                        char_count INTEGER DEFAULT 0,
                        created_at REAL NOT NULL,
                        updated_at REAL NOT NULL
                    );

                    CREATE TABLE IF NOT EXISTS chunks (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        doc_id INTEGER NOT NULL,
                        chunk_idx INTEGER NOT NULL,
                        text TEXT NOT NULL,
                        tokens_blob BLOB NOT NULL,
                        embedding_blob BLOB,
                        token_count INTEGER NOT NULL,
                        created_at REAL NOT NULL,
                        FOREIGN KEY(doc_id) REFERENCES documents(id) ON DELETE CASCADE
                    );

                    CREATE TABLE IF NOT EXISTS chat_history (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        role TEXT NOT NULL,
                        content TEXT NOT NULL,
                        timestamp REAL NOT NULL
                    );

                    CREATE TABLE IF NOT EXISTS system_meta (
                        key TEXT PRIMARY KEY,
                        value TEXT NOT NULL,
                        updated_at REAL NOT NULL
                    );

                    CREATE INDEX IF NOT EXISTS idx_chunks_doc_id ON chunks(doc_id);
                    CREATE INDEX IF NOT EXISTS idx_docs_source ON documents(source);
                """)
        finally:
            if not self.is_memory_db:
                conn.close()

    def save_document_and_chunks(
        self,
        source: str,
        title: str,
        raw_text: str,
        chunks: List[Dict],
        embeddings: Optional[torch.Tensor] = None
    ) -> int:
        now = time.time()
        conn = self._get_connection()
        try:
            with conn:
                cursor = conn.execute(
                    """
                    INSERT INTO documents (source, title, char_count, created_at, updated_at)
                    VALUES (?, ?, ?, ?, ?)
                    ON CONFLICT(source) DO UPDATE SET
                        title = excluded.title,
                        char_count = excluded.char_count,
                        updated_at = excluded.updated_at
                    RETURNING id;
                    """,
                    (source, title, len(raw_text), now, now)
                )
                doc_id = cursor.fetchone()[0]

                conn.execute("DELETE FROM chunks WHERE doc_id = ?;", (doc_id,))

                chunk_rows = []
                for idx, c in enumerate(chunks):
                    token_ids = np.array(c.get("tokens", []), dtype=np.int32)
                    tokens_blob = token_ids.tobytes()

                    emb_blob = None
                    if embeddings is not None and idx < len(embeddings):
                        emb_np = embeddings[idx].detach().cpu().numpy().astype(np.float32)
                        emb_blob = emb_np.tobytes()

                    chunk_rows.append((
                        doc_id,
                        idx,
                        c.get("text", ""),
                        tokens_blob,
                        emb_blob,
                        len(token_ids),
                        now
                    ))

                conn.executemany(
                    """
                    INSERT INTO chunks (doc_id, chunk_idx, text, tokens_blob, embedding_blob, token_count, created_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?);
                    """,
                    chunk_rows
                )
            return doc_id
        finally:
            if not self.is_memory_db:
                conn.close()

    def load_all_chunks_and_embeddings(
        self,
        device: torch.device
    ) -> Tuple[List[Dict], Optional[torch.Tensor]]:
        conn = self._get_connection()
        try:
            cursor = conn.execute("""
                SELECT c.id, c.doc_id, c.chunk_idx, c.text, c.tokens_blob, c.embedding_blob,
                       d.source, d.title
                FROM chunks c
                JOIN documents d ON c.doc_id = d.id
                ORDER BY c.doc_id, c.chunk_idx ASC;
            """)
            rows = cursor.fetchall()
        finally:
            if not self.is_memory_db:
                conn.close()

        if not rows:
            return [], None

        chunks = []
        embedding_list = []

        for row in rows:
            raw_tokens = row["tokens_blob"]
            if raw_tokens:
                tokens = np.frombuffer(raw_tokens, dtype=np.int32).tolist()
            else:
                tokens = []

            chunk_dict = {
                "chunk_id": row["id"],
                "doc_id": row["doc_id"],
                "chunk_idx": row["chunk_idx"],
                "text": row["text"],
                "tokens": tokens,
                "source": row["source"],
                "title": row["title"]
            }
            chunks.append(chunk_dict)

            raw_emb = row["embedding_blob"]
            if raw_emb:
                try:
                    emb_np = np.frombuffer(raw_emb, dtype=np.float32)
                    embedding_list.append(emb_np)
                except Exception:
                    pass

        embeddings_tensor = None
        if embedding_list and len(embedding_list) == len(chunks):
            try:
                emb_array = np.vstack(embedding_list)
                embeddings_tensor = torch.from_numpy(emb_array).to(device)
            except Exception:
                embeddings_tensor = None

        return chunks, embeddings_tensor

    def save_chat_message(self, role: str, content: str):
        conn = self._get_connection()
        try:
            with conn:
                conn.execute(
                    "INSERT INTO chat_history (role, content, timestamp) VALUES (?, ?, ?);",
                    (role, content, time.time())
                )
        except Exception:
            pass
        finally:
            if not self.is_memory_db:
                conn.close()

    def get_stats(self) -> Dict:
        conn = self._get_connection()
        try:
            num_docs = conn.execute("SELECT COUNT(*) FROM documents;").fetchone()[0]
            num_chunks = conn.execute("SELECT COUNT(*) FROM chunks;").fetchone()[0]
            total_tokens = conn.execute("SELECT COALESCE(SUM(token_count), 0) FROM chunks;").fetchone()[0]
            sources = [r[0] for r in conn.execute("SELECT source FROM documents ORDER BY source ASC;").fetchall()]
        except Exception:
            num_docs, num_chunks, total_tokens, sources = 0, 0, 0, []
        finally:
            if not self.is_memory_db:
                conn.close()

        db_size_mb = 0.0
        if not self.is_memory_db and self.db_path.exists():
            try:
                db_size_mb = round(self.db_path.stat().st_size / (1024 * 1024), 3)
            except Exception:
                db_size_mb = 0.0

        db_type = "SQLite (In-Memory)" if self.is_memory_db else "SQLite (WAL + Memory-Mapped)"
        db_path_str = ":memory:" if self.is_memory_db else str(self.db_path.resolve())

        return {
            "database_type": db_type,
            "database_path": db_path_str,
            "database_size_mb": db_size_mb,
            "total_documents": num_docs,
            "total_chunks": num_chunks,
            "total_tokens": int(total_tokens),
            "sources": sources
        }

    def clear_all(self):
        conn = self._get_connection()
        try:
            with conn:
                conn.execute("DELETE FROM chunks;")
                conn.execute("DELETE FROM documents;")
                conn.execute("DELETE FROM chat_history;")
                conn.execute("DELETE FROM system_meta;")
                if not self.is_memory_db:
                    conn.execute("VACUUM;")
        except Exception:
            pass
        finally:
            if not self.is_memory_db:
                conn.close()
