import time
import math
from pathlib import Path
from typing import List, Dict, Optional, Union
import torch
from torch.utils.data import DataLoader

from config import TrainConfig, PathConfig
from tokenizer import ByteLevelTokenizer
from model import TiwutNeuralAI
from dataset import WebScraper, DocumentReader, TextChunker, NeuralTextDataset
from memory_store import RAMNeuralMemoryBank


class NeuralTrainer:

    def __init__(
        self,
        model: TiwutNeuralAI,
        tokenizer: ByteLevelTokenizer,
        memory_bank: RAMNeuralMemoryBank,
        paths: PathConfig,
        train_config: TrainConfig,
        device: torch.device
    ):
        self.model = model.to(device)
        self.tokenizer = tokenizer
        self.memory_bank = memory_bank
        self.paths = paths
        self.train_config = train_config
        self.device = device

    def train_on_sources(
        self,
        urls: Optional[List[str]] = None,
        files: Optional[List[Union[str, Path]]] = None,
        directories: Optional[List[Union[str, Path]]] = None,
        epochs: Optional[int] = None,
        learning_rate: Optional[float] = None
    ) -> Dict:
        epochs = epochs or self.train_config.default_epochs
        lr = learning_rate or self.train_config.learning_rate
        start_time = time.time()

        print("\n" + "=" * 65)
        print(f"  🧠 Tiwut-AI Neural Training Pipeline (macOS M4 Accelerated)")
        print("=" * 65 + "\n")

        raw_documents = []

        print("[1/4] 📥 Ingesting data sources...")
        if urls:
            for url in urls:
                print(f"  🌐 Scraping URL: {url}")
                doc = WebScraper.scrape_url(url)
                if doc.get("content"):
                    raw_documents.append(doc)
                    print(f"     ✓ Extracted {len(doc['content']):,} characters from '{url}'")
                else:
                    err = doc.get("error", "Unknown error")
                    print(f"     ❌ Failed to scrape '{url}': {err}")

        if files:
            for file_path in files:
                p = Path(file_path)
                print(f"  📄 Ingesting document: {p.name}")
                doc = DocumentReader.read_file(p)
                if doc.get("content"):
                    raw_documents.append(doc)
                    print(f"     ✓ Loaded {len(doc['content']):,} characters from '{p.name}'")
                else:
                    err = doc.get("error", "Empty or unreadable")
                    print(f"     ❌ Failed to load '{p.name}': {err}")

        if directories:
            for d in directories:
                p = Path(d)
                print(f"  📁 Ingesting directory: {p.name} (recursive)")
                docs = DocumentReader.read_directory(p, recursive=True)
                print(f"     ✓ Loaded {len(docs)} text documents from '{p.name}'")
                raw_documents.extend(docs)

        if not raw_documents:
            print("\n❌ No valid documents found to train on.")
            return {"status": "error", "message": "No valid documents"}

        combined_text = "\n\n".join(d["content"] for d in raw_documents)

        print("\n[2/4] 🔡 Expanding Neural Tokenizer Vocabulary...")
        old_vocab_size = self.tokenizer.vocab_size
        added_tokens = self.tokenizer.train_on_text(combined_text, max_new_subwords=512)
        new_vocab_size = self.tokenizer.vocab_size
        print(f"  ✓ Vocab expanded: {old_vocab_size} -> {new_vocab_size} tokens")

        if new_vocab_size > old_vocab_size:
            self.model.resize_token_embeddings(new_vocab_size)
            print(f"  ✓ Model embedding matrix dynamically resized to {new_vocab_size} tokens")

        print("\n[3/4] ✂️ Generating Neural Chunk Datasets...")
        all_chunks = []
        for doc in raw_documents:
            chunks = TextChunker.chunk_text(
                doc["content"],
                self.tokenizer,
                chunk_size=self.train_config.chunk_size,
                overlap=self.train_config.chunk_overlap,
                source_meta=doc
            )
            all_chunks.extend(chunks)

        total_tokens = sum(len(c["tokens"]) for c in all_chunks)
        print(f"  ✓ Created {len(all_chunks):,} chunks ({total_tokens:,} total tokens)")

        if len(all_chunks) == 0:
            print("  ⚠️ Documents were too short to form chunks.")
            return {"status": "error", "message": "No valid chunks"}

        dataset = NeuralTextDataset(
            all_chunks,
            max_seq_len=self.train_config.chunk_size,
            pad_token_id=self.tokenizer.pad_token_id
        )

        batch_size = min(self.train_config.batch_size, max(1, len(dataset)))
        dataloader = DataLoader(
            dataset,
            batch_size=batch_size,
            shuffle=True,
            drop_last=False
        )

        print(f"\n[4/4] ⚡ Training Neural Weights on {self.device.type.upper()} Matrix Engine ({epochs} epochs)...")
        self.model.train()

        optimizer = torch.optim.AdamW(
            self.model.parameters(),
            lr=lr,
            weight_decay=self.train_config.weight_decay
        )

        total_steps = epochs * len(dataloader)
        scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(
            optimizer,
            T_max=max(1, total_steps),
            eta_min=self.train_config.min_learning_rate
        )

        final_loss = 0.0
        tokens_processed = 0

        try:
            for epoch in range(1, epochs + 1):
                epoch_loss = 0.0
                epoch_tokens = 0
                epoch_start = time.time()

                for batch_idx, (input_ids, labels) in enumerate(dataloader):
                    input_ids = input_ids.to(self.device)
                    labels = labels.to(self.device)

                    optimizer.zero_grad()
                    logits, loss, _ = self.model(input_ids, labels=labels)

                    if loss is not None and not torch.isnan(loss):
                        loss.backward()
                        torch.nn.utils.clip_grad_norm_(self.model.parameters(), self.train_config.grad_clip)
                        optimizer.step()
                        scheduler.step()

                        batch_tokens_count = (input_ids != self.tokenizer.pad_token_id).sum().item()
                        epoch_loss += loss.item()
                        epoch_tokens += batch_tokens_count
                        tokens_processed += batch_tokens_count

                avg_loss = epoch_loss / max(1, len(dataloader))
                final_loss = avg_loss
                perplexity = math.exp(min(20, avg_loss))
                epoch_time = time.time() - epoch_start
                speed = epoch_tokens / max(0.001, epoch_time)

                current_lr = scheduler.get_last_lr()[0]
                print(
                    f"  Epoch {epoch:2d}/{epochs:2d} | "
                    f"Loss: {avg_loss:.4f} | "
                    f"Perplexity: {perplexity:6.2f} | "
                    f"Speed: {speed:,.0f} tok/s | "
                    f"LR: {current_lr:.2e}"
                )
        except KeyboardInterrupt:
            print("\n⚠️ Training interrupted by user. Saving current progress...")

        print("\n💾 Persisting Neural Weights & Loading Memory into High-Speed Database & RAM...")
        self.save_model_and_tokenizer()

        for doc in raw_documents:
            doc_chunks = TextChunker.chunk_text(
                doc["content"],
                self.tokenizer,
                chunk_size=self.train_config.chunk_size,
                overlap=self.train_config.chunk_overlap,
                source_meta=doc
            )
            self.memory_bank.add_document_and_chunks(
                source=doc["source"],
                title=doc["title"],
                raw_text=doc.get("raw_text", doc["content"]),
                chunks=doc_chunks,
                model=self.model
            )

        total_elapsed = time.time() - start_time
        print(f"\n🎉 Training complete in {total_elapsed:.2f}s!")
        print(f"   • Final Loss: {final_loss:.4f}")
        print(f"   • Database: {self.memory_bank.metadata.get('db_size_mb', 0.0)} MB (SQLite WAL)")
        print(f"   • RAM Memory Bank: {self.memory_bank.metadata['total_chunks']} chunks stored ({self.memory_bank.metadata['ram_size_mb']} MB in RAM)")
        print(f"   • Total Active Tokens: {self.memory_bank.metadata['total_tokens']:,}")
        print("=" * 65 + "\n")

        return {
            "status": "success",
            "epochs": epochs,
            "final_loss": final_loss,
            "total_tokens": total_tokens,
            "elapsed_seconds": round(total_elapsed, 2),
            "ram_size_mb": self.memory_bank.metadata["ram_size_mb"]
        }

    def save_model_and_tokenizer(self):
        self.paths.base_dir.mkdir(parents=True, exist_ok=True)
        checkpoint = {
            "model_state_dict": self.model.state_dict(),
            "config": self.model.config,
            "timestamp": time.time()
        }
        torch.save(checkpoint, self.paths.checkpoint_file)
        self.tokenizer.save(self.paths.tokenizer_file)
