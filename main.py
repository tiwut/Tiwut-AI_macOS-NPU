#!/usr/bin/env python3

import sys
import os
import json
import argparse
from pathlib import Path

_venv_python = Path(__file__).resolve().parent / ".venv" / "bin" / "python3"
if _venv_python.exists() and sys.executable != str(_venv_python):
    try:
        import torch
    except ImportError:
        os.execv(str(_venv_python), [str(_venv_python)] + sys.argv)

import torch

from config import AppConfig, ModelConfig, TrainConfig, PathConfig, HardwareManager
from database import HighSpeedNeuralDatabase
from tokenizer import ByteLevelTokenizer
from model import TiwutNeuralAI
from memory_store import RAMNeuralMemoryBank
from trainer import NeuralTrainer
from chat import ChatEngine


def create_or_load_system():
    app_config = AppConfig.load()
    device = HardwareManager.get_optimal_device()
    paths = app_config.paths

    db = HighSpeedNeuralDatabase(
        db_path=paths.db_file,
        mmap_size_mb=app_config.database.mmap_size_mb
    )

    tokenizer = ByteLevelTokenizer(vocab_file=paths.tokenizer_file)

    if paths.checkpoint_file.exists():
        try:
            checkpoint = torch.load(paths.checkpoint_file, map_location=device, weights_only=False)
            model_cfg = checkpoint.get("config", app_config.model)
            model = TiwutNeuralAI(model_cfg).to(device)
            model.load_state_dict(checkpoint["model_state_dict"])
        except Exception:
            model = TiwutNeuralAI(app_config.model).to(device)
    else:
        model = TiwutNeuralAI(app_config.model).to(device)

    memory_bank = RAMNeuralMemoryBank(db=db, device=device, paths=paths)

    return model, tokenizer, memory_bank, app_config, device


def normalize_sys_argv():
    normalized = []
    supported_flags = {
        "-help": "--help",
        "-h": "--help",
        "-status": "--status",
        "-s": "--status",
        "-config": "--config",
        "-chat": "--chat",
        "-c": "--chat",
        "-ask": "--ask",
        "-a": "--ask",
        "-train": "--train",
        "-t": "--train",
        "-reset": "--reset",
        "-url": "--url",
        "-u": "--url",
        "-file": "--file",
        "-f": "--file",
        "-dir": "--dir",
        "-d": "--dir",
        "-epochs": "--epochs",
        "-e": "--epochs",
        "-lr": "--lr",
        "-batch": "--batch",
        "-b": "--batch"
    }

    for arg in sys.argv[1:]:
        if arg in supported_flags:
            normalized.append(supported_flags[arg])
        else:
            normalized.append(arg)

    return normalized


def show_custom_help():
    hw = HardwareManager.get_hardware_info()
    chip = hw.get("chip_name", "Apple Silicon")
    dev = hw.get("device", "mps").upper()

    print("""
========================================================================
  🧠 Tiwut-AI: macOS M4 Hardware-Accelerated Neural Network AI CLI
========================================================================
  Architecture: Apple Silicon M4 Matrix Engine & MPS Acceleration
  Memory:       Unified RAM Pre-Loaded Neural State (Zero Disk Latency)
  Database:     High-Speed SQLite WAL Engine (Memory-Mapped I/O)
  Config File:  config.json

USAGE:
  python3 main.py -help
  python3 main.py -status
  python3 main.py -config
  python3 main.py -chat
  python3 main.py -ask "<your question>"
  python3 main.py -train -url <website_url> [-epochs <n>]
  python3 main.py -train -file <path_to_txt_file> [-epochs <n>]
  python3 main.py -train -dir <path_to_folder> [-epochs <n>]
  python3 main.py -reset

OPTIONS:
  -help, --help            Show this help manual
  -status, --status        Display neural network parameters, M4 status & RAM state
  -config, --config        View active system JSON configuration (config.json)
  -chat, --chat            Launch interactive real-time streaming chat CLI
  -ask, --ask "<query>"    Ask a single question and get an instant neural response
  -train, --train          Train/fine-tune the neural network on documents or websites
  -url, --url <URL>        Web URL to scrape and train into neural weights
  -file, --file <PATH>     Local text document (.txt, .md, .csv) to train
  -dir, --dir <PATH>       Directory of documents to recursively train
  -epochs, --epochs <N>    Number of training epochs (default from config.json)
  -lr, --lr <FLOAT>        Learning rate (default from config.json)
  -reset, --reset          Reset neural network weights, database & RAM memory

EXAMPLES:
  1. Train on a website URL:
  python3 main.py -train -url https://en.wikipedia.org/wiki/Artificial_intelligence -epochs 10

  2. Train on a local txt file:
  python3 main.py -train -file notes.txt -epochs 15

  3. Train on a whole directory of text documents:
  python3 main.py -train -dir ./documents -epochs 8

  4. Start interactive chat with streaming tokens:
  python3 main.py -chat

  5. One-shot question answering:
  python3 main.py -ask "What is Artificial Intelligence?"

  6. View database & hardware status:
  python3 main.py -status

  7. View active configuration JSON:
  python3 main.py -config
========================================================================
""")


def main():
    normalized_args = normalize_sys_argv()

    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--help", action="store_true")
    parser.add_argument("--status", action="store_true")
    parser.add_argument("--config", action="store_true")
    parser.add_argument("--chat", action="store_true")
    parser.add_argument("--ask", type=str, default=None)
    parser.add_argument("--train", action="store_true")
    parser.add_argument("--url", type=str, default=None)
    parser.add_argument("--file", type=str, default=None)
    parser.add_argument("--dir", type=str, default=None)
    parser.add_argument("--epochs", type=int, default=None)
    parser.add_argument("--lr", type=float, default=None)
    parser.add_argument("--batch", type=int, default=None)
    parser.add_argument("--reset", action="store_true")

    if not normalized_args or "--help" in normalized_args:
        show_custom_help()
        sys.exit(0)

    try:
        args = parser.parse_args(normalized_args)
    except Exception as e:
        print(f"Error parsing arguments: {e}")
        show_custom_help()
        sys.exit(1)

    model, tokenizer, memory_bank, app_config, device = create_or_load_system()
    paths = app_config.paths

    if args.config:
        config_file = Path(__file__).resolve().parent / "config.json"
        print(f"\nActive Configuration File ({config_file}):\n")
        if config_file.exists():
            with open(config_file, "r", encoding="utf-8") as f:
                print(f.read().strip())
        else:
            print(json.dumps(app_config.__dict__, indent=2, default=str))
        print()
        return

    if args.reset:
        confirm = input("⚠️ Are you sure you want to reset the neural network and database? (y/N): ").strip()
        if confirm.lower() in ["y", "yes"]:
            memory_bank.clear()
            if paths.checkpoint_file.exists():
                paths.checkpoint_file.unlink()
            if paths.tokenizer_file.exists():
                paths.tokenizer_file.unlink()
            if paths.db_file != Path(":memory:"):
                for db_extra in [paths.db_file, paths.db_file.with_suffix(".db-wal"), paths.db_file.with_suffix(".db-shm")]:
                    if db_extra.exists():
                        db_extra.unlink()
            print("✓ Neural network weights and database have been successfully reset.")
        else:
            print("Action cancelled.")
        return

    if args.status:
        chat_engine = ChatEngine(model, tokenizer, memory_bank, device, app_config=app_config)
        chat_engine._show_status()
        chat_engine._show_memory()
        return

    if args.train:
        urls = [args.url] if args.url else None
        files = [args.file] if args.file else None
        dirs = [args.dir] if args.dir else None

        if not urls and not files and not dirs:
            print("❌ Error: -train requires a target source. Use -url <url>, -file <path>, or -dir <path>.")
            print("Example: python3 main.py -train -url https://example.com -epochs 10")
            sys.exit(1)

        epochs = args.epochs if args.epochs is not None else app_config.training.default_epochs
        lr = args.lr if args.lr is not None else app_config.training.learning_rate
        batch_size = args.batch if args.batch is not None else app_config.training.batch_size

        train_cfg = TrainConfig(
            default_epochs=epochs,
            learning_rate=lr,
            batch_size=batch_size,
            chunk_size=app_config.training.chunk_size,
            chunk_overlap=app_config.training.chunk_overlap,
            weight_decay=app_config.training.weight_decay,
            grad_clip=app_config.training.grad_clip,
            min_learning_rate=app_config.training.min_learning_rate
        )

        trainer = NeuralTrainer(
            model=model,
            tokenizer=tokenizer,
            memory_bank=memory_bank,
            paths=paths,
            train_config=train_cfg,
            device=device
        )

        trainer.train_on_sources(
            urls=urls,
            files=files,
            directories=dirs,
            epochs=epochs,
            learning_rate=lr
        )
        return

    if args.ask is not None:
        chat_engine = ChatEngine(model, tokenizer, memory_bank, device, app_config=app_config)
        chat_engine.ask_single(args.ask)
        return

    if args.chat:
        chat_engine = ChatEngine(model, tokenizer, memory_bank, device, app_config=app_config)
        chat_engine.start_interactive_session()
        return

    show_custom_help()


if __name__ == "__main__":
    main()
