import json
from pathlib import Path
from typing import List, Dict, Union, Optional


class ByteLevelTokenizer:

    def __init__(self, vocab_file: Optional[Union[str, Path]] = None, max_vocab_size: int = 16384):
        self.max_vocab_size = max_vocab_size
        self.pad_token = "<pad>"
        self.bos_token = "<bos>"
        self.eos_token = "<eos>"
        self.unk_token = "<unk>"
        self.mask_token = "<mask>"

        self.special_tokens = [
            "<pad>",
            "<bos>",
            "<eos>",
            "<unk>",
            "<mask>",
            "<s>",
            "</s>",
            "<sep>"
        ]

        self.token_to_id: Dict[str, int] = {}
        self.id_to_token: Dict[int, str] = {}
        self._init_base_vocab()

        if vocab_file and Path(vocab_file).exists():
            self.load(vocab_file)

    def _init_base_vocab(self):
        self.token_to_id.clear()
        self.id_to_token.clear()

        for idx, token in enumerate(self.special_tokens):
            self.token_to_id[token] = idx
            self.id_to_token[idx] = token

        for b in range(256):
            byte_token = f"<byte_{b:02x}>"
            idx = len(self.token_to_id)
            self.token_to_id[byte_token] = idx
            self.id_to_token[idx] = byte_token

    @property
    def pad_token_id(self) -> int:
        return self.token_to_id.get("<pad>", 0)

    @property
    def unk_token_id(self) -> int:
        return self.token_to_id.get("<unk>", 3)

    @property
    def bos_token_id(self) -> int:
        return self.token_to_id.get("<bos>", self.token_to_id.get("<s>", 1))

    @property
    def eos_token_id(self) -> int:
        return self.token_to_id.get("<eos>", self.token_to_id.get("</s>", 2))

    @property
    def mask_token_id(self) -> int:
        return self.token_to_id.get("<mask>", 4)

    @property
    def vocab_size(self) -> int:
        return len(self.token_to_id)

    def train_on_text(self, text: str, max_new_subwords: int = 512) -> int:
        if not text:
            return 0

        words = text.split()
        word_freq: Dict[str, int] = {}
        for w in words:
            clean_w = w.strip()
            if 2 <= len(clean_w) <= 24:
                word_freq[clean_w] = word_freq.get(clean_w, 0) + 1

        sorted_words = sorted(word_freq.items(), key=lambda x: x[1], reverse=True)
        added_count = 0

        for word, freq in sorted_words:
            if freq >= 2 and word not in self.token_to_id:
                if len(self.token_to_id) >= self.max_vocab_size:
                    break
                idx = len(self.token_to_id)
                self.token_to_id[word] = idx
                self.id_to_token[idx] = word
                added_count += 1
                if added_count >= max_new_subwords:
                    break

        return added_count

    def encode(self, text: str, add_special_tokens: bool = False) -> List[int]:
        if not text:
            return [self.bos_token_id, self.eos_token_id] if add_special_tokens else []

        tokens: List[int] = []
        if add_special_tokens:
            tokens.append(self.bos_token_id)

        words = text.split(" ")
        num_words = len(words)

        for i, word in enumerate(words):
            if not word:
                if i < num_words - 1:
                    space_b = ord(" ")
                    tokens.append(self.token_to_id.get(f"<byte_{space_b:02x}>", self.unk_token_id))
                continue

            if word in self.token_to_id:
                tokens.append(self.token_to_id[word])
            else:
                raw_bytes = word.encode("utf-8", errors="replace")
                for b in raw_bytes:
                    byte_str = f"<byte_{b:02x}>"
                    tokens.append(self.token_to_id.get(byte_str, self.unk_token_id))

            if i < num_words - 1:
                space_b = ord(" ")
                tokens.append(self.token_to_id.get(f"<byte_{space_b:02x}>", self.unk_token_id))

        if add_special_tokens:
            tokens.append(self.eos_token_id)

        return tokens

    def decode(self, token_ids: List[int], skip_special_tokens: bool = True) -> str:
        if not token_ids:
            return ""

        byte_buffer = bytearray()
        special_ids = {self.pad_token_id, self.unk_token_id, self.bos_token_id, self.eos_token_id, self.mask_token_id}
        result_parts = []

        for tid in token_ids:
            if skip_special_tokens and tid in special_ids:
                continue

            token = self.id_to_token.get(tid)
            if not token:
                continue

            if token.startswith("<byte_") and token.endswith(">") and len(token) == 9:
                try:
                    b_val = int(token[6:8], 16)
                    byte_buffer.append(b_val)
                    continue
                except ValueError:
                    pass

            if byte_buffer:
                result_parts.append(byte_buffer.decode("utf-8", errors="replace"))
                byte_buffer.clear()

            result_parts.append(token)

        if byte_buffer:
            result_parts.append(byte_buffer.decode("utf-8", errors="replace"))

        return "".join(result_parts)

    def save(self, file_path: Union[str, Path]):
        path = Path(file_path)
        path.parent.mkdir(parents=True, exist_ok=True)
        data = {
            "token_to_id": self.token_to_id,
            "max_vocab_size": self.max_vocab_size,
        }
        with open(path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)

    def load(self, file_path: Union[str, Path]):
        path = Path(file_path)
        if not path.exists():
            return
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)

        self.token_to_id = data.get("token_to_id", {})
        self.id_to_token = {int(v): k for k, v in self.token_to_id.items()}
        self.max_vocab_size = data.get("max_vocab_size", self.max_vocab_size)
