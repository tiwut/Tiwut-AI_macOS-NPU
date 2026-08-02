import os
import re
from pathlib import Path
from typing import List, Dict, Union, Optional
import requests
from bs4 import BeautifulSoup
import torch
from torch.utils.data import Dataset

from tokenizer import ByteLevelTokenizer


class WebScraper:

    HEADERS = {
        "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,text/plain;q=0.8,*/*;q=0.7",
        "Accept-Language": "en-US,en;q=0.9",
    }

    @classmethod
    def scrape_url(cls, url: str, timeout: int = 15) -> Dict[str, str]:
        if not url.startswith(("http://", "https://")):
            url = "https://" + url

        try:
            response = requests.get(url, headers=cls.HEADERS, timeout=timeout, allow_redirects=True)
            response.raise_for_status()

            if response.encoding is None or response.encoding.lower() == "iso-8859-1":
                response.encoding = response.apparent_encoding or "utf-8"

            content_type = response.headers.get("Content-Type", "").lower()
            if "text/plain" in content_type:
                clean_text = response.text.strip()
                title = url.split("/")[-1] or url
                return {
                    "source": url,
                    "title": title,
                    "content": clean_text,
                    "char_count": str(len(clean_text))
                }

            soup = BeautifulSoup(response.text, "html.parser")

            for tag in soup(["script", "style", "nav", "footer", "header", "aside", "noscript", "svg", "form", "iframe"]):
                tag.decompose()

            title_tag = soup.find("title")
            title = title_tag.get_text().strip() if title_tag else url

            paragraphs = []
            for element in soup.find_all(["h1", "h2", "h3", "h4", "p", "li", "blockquote", "pre", "code"]):
                text = element.get_text().strip()
                if text and len(text) > 5:
                    paragraphs.append(text)

            if not paragraphs:
                body = soup.find("body")
                if body:
                    clean_text = body.get_text(separator="\n").strip()
                else:
                    clean_text = soup.get_text(separator="\n").strip()
            else:
                clean_text = "\n\n".join(paragraphs)

            clean_text = re.sub(r"\n{3,}", "\n\n", clean_text)
            clean_text = re.sub(r"[ \t]{2,}", " ", clean_text)

            return {
                "source": url,
                "title": title,
                "content": clean_text,
                "char_count": str(len(clean_text))
            }

        except Exception as e:
            return {
                "source": url,
                "title": url,
                "content": "",
                "error": str(e)
            }


class DocumentReader:

    ENCODINGS = ["utf-8", "utf-8-sig", "latin-1", "cp1252"]

    @classmethod
    def is_binary(cls, file_path: Path) -> bool:
        try:
            with open(file_path, "rb") as f:
                chunk = f.read(1024)
                if b"\x00" in chunk:
                    return True
        except Exception:
            return True
        return False

    @classmethod
    def read_file(cls, file_path: Union[str, Path]) -> Dict[str, str]:
        path = Path(file_path).resolve()
        if not path.exists() or not path.is_file():
            return {"source": str(path), "title": str(path), "content": "", "error": "File not found"}

        if cls.is_binary(path):
            return {"source": str(path), "title": path.name, "content": "", "error": "Binary file skipped"}

        content = ""
        for enc in cls.ENCODINGS:
            try:
                with open(path, "r", encoding=enc, errors="strict") as f:
                    content = f.read()
                break
            except (UnicodeDecodeError, LookupError):
                continue

        if not content:
            try:
                with open(path, "r", encoding="utf-8", errors="replace") as f:
                    content = f.read()
            except Exception as e:
                return {"source": str(path), "title": path.name, "content": "", "error": str(e)}

        clean_text = re.sub(r"\r\n", "\n", content).strip()
        return {
            "source": str(path),
            "title": path.name,
            "content": clean_text,
            "char_count": str(len(clean_text))
        }

    @classmethod
    def read_directory(cls, dir_path: Union[str, Path], recursive: bool = True) -> List[Dict[str, str]]:
        path = Path(dir_path).resolve()
        if not path.exists() or not path.is_dir():
            return []

        supported_exts = {".txt", ".md", ".markdown", ".csv", ".tsv", ".json", ".py", ".js", ".html", ".log", ".rst"}
        documents = []

        glob_func = path.rglob if recursive else path.glob
        for file_path in glob_func("*"):
            if file_path.is_file() and file_path.suffix.lower() in supported_exts:
                doc = cls.read_file(file_path)
                if doc.get("content"):
                    documents.append(doc)

        return documents


class TextChunker:

    @classmethod
    def chunk_text(
        cls,
        text: str,
        tokenizer: ByteLevelTokenizer,
        chunk_size: int = 256,
        overlap: int = 64,
        source_meta: Optional[Dict] = None
    ) -> List[Dict]:
        if not text:
            return []

        source_meta = source_meta or {}
        source = source_meta.get("source", "unknown")
        title = source_meta.get("title", "Untitled")

        paragraphs = text.split("\n\n")
        raw_sections = []
        for p in paragraphs:
            p_clean = p.strip()
            if len(p_clean) > 800:
                sentences = re.split(r"(?<=[.!?])\s+", p_clean)
                buffer = ""
                for s in sentences:
                    if len(buffer) + len(s) < 600:
                        buffer += (" " if buffer else "") + s
                    else:
                        if buffer:
                            raw_sections.append(buffer)
                        buffer = s
                if buffer:
                    raw_sections.append(buffer)
            elif p_clean:
                raw_sections.append(p_clean)

        chunks = []
        chunk_idx = 0
        step = max(1, chunk_size - overlap)

        for sec in raw_sections:
            sec_tokens = tokenizer.encode(sec, add_special_tokens=False)
            if not sec_tokens:
                continue

            if len(sec_tokens) <= chunk_size:
                chunks.append({
                    "chunk_id": f"{source}_{chunk_idx}",
                    "chunk_idx": chunk_idx,
                    "source": source,
                    "title": title,
                    "text": sec,
                    "tokens": sec_tokens
                })
                chunk_idx += 1
            else:
                for i in range(0, len(sec_tokens), step):
                    window = sec_tokens[i:i + chunk_size]
                    if len(window) < 10:
                        continue
                    window_text = tokenizer.decode(window, skip_special_tokens=True)
                    chunks.append({
                        "chunk_id": f"{source}_{chunk_idx}",
                        "chunk_idx": chunk_idx,
                        "source": source,
                        "title": title,
                        "text": window_text,
                        "tokens": window
                    })
                    chunk_idx += 1

        return chunks


class NeuralTextDataset(Dataset):

    def __init__(self, chunks: List[Dict], max_seq_len: int = 256, pad_token_id: int = 0):
        self.samples = []
        self.max_seq_len = max_seq_len
        self.pad_token_id = pad_token_id

        for c in chunks:
            toks = c.get("tokens", [])
            if len(toks) > 1:
                self.samples.append(toks[:max_seq_len])

    def __len__(self) -> int:
        return len(self.samples)

    def __getitem__(self, idx: int) -> Tuple[torch.Tensor, torch.Tensor]:
        tokens = self.samples[idx]
        seq_len = len(tokens)

        if seq_len < self.max_seq_len:
            padded = tokens + [self.pad_token_id] * (self.max_seq_len - seq_len)
            labels = tokens[1:] + [self.pad_token_id] + [-100] * (self.max_seq_len - seq_len)
        else:
            padded = tokens[:self.max_seq_len]
            labels = tokens[1:self.max_seq_len] + [self.pad_token_id]

        input_tensor = torch.tensor(padded, dtype=torch.long)
        label_tensor = torch.tensor(labels, dtype=torch.long)
        return input_tensor, label_tensor
