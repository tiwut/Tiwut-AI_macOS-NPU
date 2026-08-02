import json
import os
import platform
import subprocess
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Dict, Any, Optional
import torch

CONFIG_FILE_PATH = Path(__file__).resolve().parent / "config.json"


@dataclass
class DatabaseConfig:
    type: str = "sqlite_wal"
    path: str = "storage/neural_brain.db"
    in_memory_cache: bool = True
    mmap_size_mb: int = 256
    wal_autocheckpoint: int = 1000


@dataclass
class PathConfig:
    base_dir: Path = Path(__file__).resolve().parent / "storage"
    checkpoint_file: Path = field(init=False)
    tokenizer_file: Path = field(init=False)
    db_file: Path = field(init=False)
    meta_file: Path = field(init=False)

    def __post_init__(self):
        self.base_dir.mkdir(parents=True, exist_ok=True)
        self.checkpoint_file = self.base_dir / "neural_weights.pt"
        self.tokenizer_file = self.base_dir / "tokenizer.json"
        self.db_file = self.base_dir / "neural_brain.db"
        self.meta_file = self.base_dir / "metadata.json"


@dataclass
class ModelConfig:
    vocab_size: int = 4096
    embed_dim: int = 256
    num_layers: int = 6
    num_heads: int = 8
    feedforward_dim: int = 1024
    max_seq_len: int = 512
    dropout: float = 0.1
    tie_weights: bool = True


@dataclass
class TrainConfig:
    batch_size: int = 16
    learning_rate: float = 3e-4
    min_learning_rate: float = 3e-5
    weight_decay: float = 0.01
    grad_clip: float = 1.0
    default_epochs: int = 10
    chunk_size: int = 256
    chunk_overlap: int = 64


@dataclass
class InferenceConfig:
    temperature: float = 0.6
    top_k: int = 40
    top_p: float = 0.9
    repetition_penalty: float = 1.15
    max_tokens: int = 200
    stream: bool = True
    memory_threshold: float = 0.10


@dataclass
class AppConfig:
    database: DatabaseConfig = field(default_factory=DatabaseConfig)
    paths: PathConfig = field(default_factory=PathConfig)
    model: ModelConfig = field(default_factory=ModelConfig)
    training: TrainConfig = field(default_factory=TrainConfig)
    inference: InferenceConfig = field(default_factory=InferenceConfig)

    @classmethod
    def load(cls, config_path: Path = CONFIG_FILE_PATH) -> "AppConfig":
        root_dir = Path(__file__).resolve().parent

        if not config_path.exists():
            instance = cls()
            instance.save(config_path)
            return instance

        try:
            with open(config_path, "r", encoding="utf-8") as f:
                data: Dict[str, Any] = json.load(f)
        except Exception:
            instance = cls()
            return instance

        try:
            db_cfg = DatabaseConfig(**data.get("database", {}))
        except Exception:
            db_cfg = DatabaseConfig()

        storage_data = data.get("storage", {})
        base_dir_str = storage_data.get("base_dir", "storage")
        base_dir = root_dir / base_dir_str
        paths_cfg = PathConfig(base_dir=base_dir)

        paths_cfg.checkpoint_file = root_dir / storage_data.get("checkpoint_file", f"{base_dir_str}/neural_weights.pt")
        paths_cfg.tokenizer_file = root_dir / storage_data.get("tokenizer_file", f"{base_dir_str}/tokenizer.json")
        
        if db_cfg.path == ":memory:":
            paths_cfg.db_file = Path(":memory:")
        else:
            paths_cfg.db_file = root_dir / db_cfg.path

        paths_cfg.meta_file = root_dir / storage_data.get("meta_file", f"{base_dir_str}/metadata.json")

        try:
            model_cfg = ModelConfig(**data.get("model", {}))
        except Exception:
            model_cfg = ModelConfig()

        try:
            train_cfg = TrainConfig(**data.get("training", {}))
        except Exception:
            train_cfg = TrainConfig()

        try:
            inf_cfg = InferenceConfig(**data.get("inference", {}))
        except Exception:
            inf_cfg = InferenceConfig()

        return cls(
            database=db_cfg,
            paths=paths_cfg,
            model=model_cfg,
            training=train_cfg,
            inference=inf_cfg
        )

    def save(self, config_path: Path = CONFIG_FILE_PATH):
        root_dir = Path(__file__).resolve().parent
        try:
            db_path_str = str(self.paths.db_file.relative_to(root_dir)) if self.paths.db_file != Path(":memory:") else ":memory:"
        except ValueError:
            db_path_str = str(self.paths.db_file)

        data = {
            "database": {
                "type": self.database.type,
                "path": db_path_str,
                "in_memory_cache": self.database.in_memory_cache,
                "mmap_size_mb": self.database.mmap_size_mb,
                "wal_autocheckpoint": self.database.wal_autocheckpoint
            },
            "storage": {
                "base_dir": str(self.paths.base_dir.relative_to(root_dir)) if self.paths.base_dir.is_relative_to(root_dir) else str(self.paths.base_dir),
                "checkpoint_file": str(self.paths.checkpoint_file.relative_to(root_dir)) if self.paths.checkpoint_file.is_relative_to(root_dir) else str(self.paths.checkpoint_file),
                "tokenizer_file": str(self.paths.tokenizer_file.relative_to(root_dir)) if self.paths.tokenizer_file.is_relative_to(root_dir) else str(self.paths.tokenizer_file),
                "meta_file": str(self.paths.meta_file.relative_to(root_dir)) if self.paths.meta_file.is_relative_to(root_dir) else str(self.paths.meta_file)
            },
            "hardware": {
                "device": "auto",
                "enable_mps_fallback": True
            },
            "model": asdict(self.model),
            "training": asdict(self.training),
            "inference": asdict(self.inference)
        }
        try:
            with open(config_path, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2)
        except Exception:
            pass


class HardwareManager:

    @staticmethod
    def get_hardware_info() -> dict:
        info = {
            "os": platform.system(),
            "os_release": platform.release(),
            "architecture": platform.machine(),
            "chip_name": "Apple Silicon",
            "device": "cpu",
            "mps_available": False,
            "unified_ram_gb": 0.0,
        }

        if platform.system() == "Darwin":
            try:
                res = subprocess.run(
                    ["sysctl", "-n", "machdep.cpu.brand_string"],
                    capture_output=True,
                    text=True,
                    check=False
                )
                if res.returncode == 0 and res.stdout.strip():
                    info["chip_name"] = res.stdout.strip()
                else:
                    res2 = subprocess.run(
                        ["sysctl", "-n", "hw.model"],
                        capture_output=True,
                        text=True,
                        check=False
                    )
                    if res2.returncode == 0:
                        info["chip_name"] = f"Apple Silicon ({res2.stdout.strip()})"
            except Exception:
                pass

            try:
                res_mem = subprocess.run(
                    ["sysctl", "-n", "hw.memsize"],
                    capture_output=True,
                    text=True,
                    check=False
                )
                if res_mem.returncode == 0:
                    mem_bytes = int(res_mem.stdout.strip())
                    info["unified_ram_gb"] = round(mem_bytes / (1024 ** 3), 2)
            except Exception:
                pass

        if torch.backends.mps.is_available():
            info["mps_available"] = True
            info["device"] = "mps"
        elif torch.cuda.is_available():
            info["device"] = "cuda"

        return info

    @classmethod
    def get_optimal_device(cls) -> torch.device:
        if torch.backends.mps.is_available():
            os.environ["PYTORCH_ENABLE_MPS_FALLBACK"] = "1"
            return torch.device("mps")
        elif torch.cuda.is_available():
            return torch.device("cuda")
        return torch.device("cpu")
