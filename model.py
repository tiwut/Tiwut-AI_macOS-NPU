import math
from typing import Optional, Tuple
import torch
import torch.nn as nn
import torch.nn.functional as F

from config import ModelConfig


class RMSNorm(nn.Module):

    def __init__(self, dim: int, eps: float = 1e-6):
        super().__init__()
        self.eps = eps
        self.weight = nn.Parameter(torch.ones(dim))

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        variance = x.pow(2).mean(-1, keepdim=True)
        return x * torch.rsqrt(variance + self.eps) * self.weight


class RotaryEmbedding(nn.Module):

    def __init__(self, dim: int, max_seq_len: int = 2048, base: int = 10000):
        super().__init__()
        self.dim = dim
        self.max_seq_len = max_seq_len
        inv_freq = 1.0 / (base ** (torch.arange(0, dim, 2).float() / dim))
        self.register_buffer("inv_freq", inv_freq, persistent=False)
        self._build_cache(max_seq_len)

    def _build_cache(self, seq_len: int):
        t = torch.arange(seq_len, dtype=torch.float32, device=self.inv_freq.device)
        freqs = torch.outer(t, self.inv_freq)
        emb = torch.cat((freqs, freqs), dim=-1)
        self.register_buffer("cos_cached", emb.cos(), persistent=False)
        self.register_buffer("sin_cached", emb.sin(), persistent=False)

    def forward(self, x: torch.Tensor, seq_len: int) -> Tuple[torch.Tensor, torch.Tensor]:
        if seq_len > self.cos_cached.shape[0] or self.cos_cached.device != x.device:
            self._build_cache(max(seq_len, self.max_seq_len))
        return self.cos_cached[:seq_len].to(x.device), self.sin_cached[:seq_len].to(x.device)


def rotate_half(x: torch.Tensor) -> torch.Tensor:
    x1 = x[..., : x.shape[-1] // 2]
    x2 = x[..., x.shape[-1] // 2 :]
    return torch.cat((-x2, x1), dim=-1)


def apply_rotary_pos_emb(q: torch.Tensor, k: torch.Tensor, cos: torch.Tensor, sin: torch.Tensor) -> Tuple[torch.Tensor, torch.Tensor]:
    cos = cos.unsqueeze(0).unsqueeze(1)
    sin = sin.unsqueeze(0).unsqueeze(1)
    q_rot = (q * cos) + (rotate_half(q) * sin)
    k_rot = (k * cos) + (rotate_half(k) * sin)
    return q_rot, k_rot


class SwiGLU(nn.Module):

    def __init__(self, in_dim: int, hidden_dim: int):
        super().__init__()
        self.w1 = nn.Linear(in_dim, hidden_dim, bias=False)
        self.w2 = nn.Linear(hidden_dim, in_dim, bias=False)
        self.w3 = nn.Linear(in_dim, hidden_dim, bias=False)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.w2(F.silu(self.w1(x)) * self.w3(x))


class CausalSelfAttention(nn.Module):

    def __init__(self, config: ModelConfig):
        super().__init__()
        self.num_heads = config.num_heads
        self.head_dim = config.embed_dim // config.num_heads
        self.embed_dim = config.embed_dim

        self.q_proj = nn.Linear(config.embed_dim, config.embed_dim, bias=False)
        self.k_proj = nn.Linear(config.embed_dim, config.embed_dim, bias=False)
        self.v_proj = nn.Linear(config.embed_dim, config.embed_dim, bias=False)
        self.out_proj = nn.Linear(config.embed_dim, config.embed_dim, bias=False)
        self.dropout = nn.Dropout(config.dropout)

    def forward(
        self,
        x: torch.Tensor,
        cos: torch.Tensor,
        sin: torch.Tensor,
        mask: Optional[torch.Tensor] = None
    ) -> torch.Tensor:
        B, T, C = x.shape

        q = self.q_proj(x).view(B, T, self.num_heads, self.head_dim).transpose(1, 2)
        k = self.k_proj(x).view(B, T, self.num_heads, self.head_dim).transpose(1, 2)
        v = self.v_proj(x).view(B, T, self.num_heads, self.head_dim).transpose(1, 2)

        q, k = apply_rotary_pos_emb(q, k, cos, sin)

        is_causal = mask is None and T > 1
        attn_out = F.scaled_dot_product_attention(
            q, k, v,
            attn_mask=mask,
            dropout_p=self.dropout.p if self.training else 0.0,
            is_causal=is_causal
        )

        attn_out = attn_out.transpose(1, 2).contiguous().view(B, T, C)
        return self.out_proj(attn_out)


class TransformerBlock(nn.Module):

    def __init__(self, config: ModelConfig):
        super().__init__()
        self.attn_norm = RMSNorm(config.embed_dim)
        self.attn = CausalSelfAttention(config)
        self.ffn_norm = RMSNorm(config.embed_dim)
        self.ffn = SwiGLU(config.embed_dim, config.feedforward_dim)
        self.dropout = nn.Dropout(config.dropout)

    def forward(
        self,
        x: torch.Tensor,
        cos: torch.Tensor,
        sin: torch.Tensor,
        mask: Optional[torch.Tensor] = None
    ) -> torch.Tensor:
        h = x + self.dropout(self.attn(self.attn_norm(x), cos, sin, mask=mask))
        out = h + self.dropout(self.ffn(self.ffn_norm(h)))
        return out


class TiwutNeuralAI(nn.Module):

    def __init__(self, config: ModelConfig):
        super().__init__()
        self.config = config

        self.tok_embeddings = nn.Embedding(config.vocab_size, config.embed_dim)
        self.rope = RotaryEmbedding(config.embed_dim // config.num_heads, max_seq_len=config.max_seq_len)

        self.layers = nn.ModuleList([
            TransformerBlock(config) for _ in range(config.num_layers)
        ])

        self.final_norm = RMSNorm(config.embed_dim)
        self.lm_head = nn.Linear(config.embed_dim, config.vocab_size, bias=False)

        if config.tie_weights:
            self.lm_head.weight = self.tok_embeddings.weight

        self.semantic_proj = nn.Sequential(
            nn.Linear(config.embed_dim, config.embed_dim),
            nn.SiLU(),
            nn.Linear(config.embed_dim, config.embed_dim, bias=False)
        )

        self.apply(self._init_weights)

    def _init_weights(self, module):
        if isinstance(module, nn.Linear):
            torch.nn.init.normal_(module.weight, mean=0.0, std=0.02)
            if module.bias is not None:
                torch.nn.init.zeros_(module.bias)
        elif isinstance(module, nn.Embedding):
            torch.nn.init.normal_(module.weight, mean=0.0, std=0.02)

    def resize_token_embeddings(self, new_vocab_size: int):
        if new_vocab_size <= self.config.vocab_size:
            return

        old_embed = self.tok_embeddings
        new_embed = nn.Embedding(new_vocab_size, self.config.embed_dim).to(old_embed.weight.device)
        self._init_weights(new_embed)

        with torch.no_grad():
            new_embed.weight[:self.config.vocab_size] = old_embed.weight

        self.tok_embeddings = new_embed

        if not self.config.tie_weights:
            old_head = self.lm_head
            new_head = nn.Linear(self.config.embed_dim, new_vocab_size, bias=False).to(old_head.weight.device)
            self._init_weights(new_head)
            with torch.no_grad():
                new_head.weight[:self.config.vocab_size] = old_head.weight
            self.lm_head = new_head
        else:
            self.lm_head.weight = self.tok_embeddings.weight

        self.config.vocab_size = new_vocab_size

    def forward(
        self,
        input_ids: torch.Tensor,
        labels: Optional[torch.Tensor] = None,
        mask: Optional[torch.Tensor] = None
    ) -> Tuple[torch.Tensor, Optional[torch.Tensor], torch.Tensor]:
        if input_ids.shape[1] > self.config.max_seq_len:
            input_ids = input_ids[:, :self.config.max_seq_len]
            if labels is not None:
                labels = labels[:, :self.config.max_seq_len]

        B, T = input_ids.shape
        x = self.tok_embeddings(input_ids)
        cos, sin = self.rope(x, T)

        for layer in self.layers:
            x = layer(x, cos, sin, mask=mask)

        hidden_states = self.final_norm(x)
        logits = self.lm_head(hidden_states)

        loss = None
        if labels is not None:
            shift_logits = logits[..., :-1, :].contiguous()
            shift_labels = labels[..., 1:].contiguous()
            loss = F.cross_entropy(
                shift_logits.view(-1, shift_logits.size(-1)),
                shift_labels.view(-1),
                ignore_index=-100
            )

        return logits, loss, hidden_states

    @torch.no_grad()
    def encode_semantic_vector(self, input_ids: torch.Tensor) -> torch.Tensor:
        self.eval()
        if input_ids.shape[1] > self.config.max_seq_len:
            input_ids = input_ids[:, :self.config.max_seq_len]

        _, _, hidden_states = self(input_ids)
        mean_pooled = hidden_states.mean(dim=1)
        latent_vector = self.semantic_proj(mean_pooled)
        normalized = F.normalize(latent_vector, p=2, dim=-1)
        return normalized

    @torch.no_grad()
    def generate_stream(
        self,
        input_ids: torch.Tensor,
        max_new_tokens: int = 150,
        temperature: float = 0.6,
        top_k: int = 40,
        top_p: float = 0.9,
        repetition_penalty: float = 1.15,
        eos_token_id: int = 3
    ):
        self.eval()
        generated = input_ids

        for _ in range(max_new_tokens):
            if generated.shape[1] > self.config.max_seq_len:
                idx_cond = generated[:, -self.config.max_seq_len:]
            else:
                idx_cond = generated

            logits, _, _ = self(idx_cond)
            next_token_logits = logits[:, -1, :].clone()

            if repetition_penalty != 1.0:
                for token_id in set(generated[0].tolist()):
                    if next_token_logits[0, token_id] > 0:
                        next_token_logits[0, token_id] /= repetition_penalty
                    else:
                        next_token_logits[0, token_id] *= repetition_penalty

            if temperature > 0.0:
                next_token_logits = next_token_logits / temperature

                if top_k > 0:
                    v, _ = torch.topk(next_token_logits, min(top_k, next_token_logits.size(-1)))
                    next_token_logits[next_token_logits < v[:, [-1]]] = -float("Inf")

                if top_p < 1.0:
                    sorted_logits, sorted_indices = torch.sort(next_token_logits, descending=True)
                    cumulative_probs = torch.cumsum(F.softmax(sorted_logits, dim=-1), dim=-1)
                    sorted_indices_to_remove = cumulative_probs > top_p
                    sorted_indices_to_remove[..., 1:] = sorted_indices_to_remove[..., :-1].clone()
                    sorted_indices_to_remove[..., 0] = 0
                    indices_to_remove = sorted_indices_to_remove.scatter(1, sorted_indices, sorted_indices_to_remove)
                    next_token_logits[indices_to_remove] = -float("Inf")

                probs = F.softmax(next_token_logits, dim=-1)
                next_token = torch.multinomial(probs, num_samples=1)
            else:
                next_token = torch.argmax(next_token_logits, dim=-1, keepdim=True)

            token_val = next_token.item()
            if token_val == eos_token_id:
                break

            yield token_val
            generated = torch.cat((generated, next_token), dim=1)
