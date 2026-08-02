import sys
import time
import re
from typing import List, Dict, Optional
import torch

from config import HardwareManager, AppConfig
from model import TiwutNeuralAI
from tokenizer import ByteLevelTokenizer
from memory_store import RAMNeuralMemoryBank


class ChatEngine:

    BOLD = "\033[1m"
    DIM = "\033[2m"
    CYAN = "\033[96m"
    GREEN = "\033[92m"
    YELLOW = "\033[93m"
    PURPLE = "\033[95m"
    RESET = "\033[0m"

    def __init__(
        self,
        model: TiwutNeuralAI,
        tokenizer: ByteLevelTokenizer,
        memory_bank: RAMNeuralMemoryBank,
        device: torch.device,
        app_config: Optional[AppConfig] = None
    ):
        self.model = model.to(device)
        self.tokenizer = tokenizer
        self.memory_bank = memory_bank
        self.device = device
        self.app_config = app_config or AppConfig.load()
        self.conversation_history: List[Dict[str, str]] = []
        self.hw_info = HardwareManager.get_hardware_info()

    def _classify_intent(self, user_query: str) -> str:
        q = user_query.strip().lower()
        if re.match(r"^(hi|hello|hey|greetings|good\s+(morning|afternoon|evening)|yo|howdy)[\s!.]*$", q):
            return "greeting"
        if any(p in q for p in ["who are you", "what are you", "your name", "what is tiwut", "what can you do"]):
            return "identity"
        if q in ["help", "/help", "what commands", "how to use"]:
            return "help"
        return "query"

    def generate_response(
        self,
        user_query: str,
        stream: bool = True,
        max_new_tokens: Optional[int] = None,
        temperature: Optional[float] = None,
        top_k: Optional[int] = None,
        top_p: Optional[float] = None
    ) -> str:
        user_query = user_query.strip()
        if not user_query:
            return ""

        max_tokens = max_new_tokens or self.app_config.inference.max_tokens
        temp = temperature if temperature is not None else self.app_config.inference.temperature
        k = top_k if top_k is not None else self.app_config.inference.top_k
        p = top_p if top_p is not None else self.app_config.inference.top_p
        rep_pen = self.app_config.inference.repetition_penalty

        intent = self._classify_intent(user_query)

        if intent == "greeting":
            reply = "Hello! I am Tiwut-AI, your macOS M4 hardware-accelerated neural assistant. How can I help you today?"
            if stream:
                for word in reply.split(" "):
                    sys.stdout.write(word + " ")
                    sys.stdout.flush()
                    time.sleep(0.015)
                sys.stdout.write("\n")
            else:
                print(reply)
            self.memory_bank.db.save_chat_message("user", user_query)
            self.memory_bank.db.save_chat_message("assistant", reply)
            return reply

        if intent == "identity":
            chip = self.hw_info.get("chip_name", "Apple Silicon")
            total_chunks = self.memory_bank.metadata.get("total_chunks", 0)
            sources = self.memory_bank.metadata.get("sources", [])
            reply = (
                f"I am Tiwut-AI, a lightweight, high-speed neural network AI running on {chip} with Metal Performance Shaders (MPS).\n"
                f"I have {total_chunks} memory chunks pre-loaded into Unified RAM from {len(sources)} source(s)."
            )
            if stream:
                for line in reply.split("\n"):
                    print(line)
                    time.sleep(0.02)
            else:
                print(reply)
            self.memory_bank.db.save_chat_message("user", user_query)
            self.memory_bank.db.save_chat_message("assistant", reply)
            return reply

        top_chunks = self.memory_bank.search_neural_memory(
            query=user_query,
            model=self.model,
            tokenizer=self.tokenizer,
            top_k=3,
            threshold=self.app_config.inference.memory_threshold
        )

        extracted_answer = None
        if top_chunks:
            extracted_answer = self.memory_bank.extract_intelligent_answer(user_query, top_chunks)

        if extracted_answer:
            source_ref = top_chunks[0][0].get("source", "Memory Bank")
            reply = f"{extracted_answer}\n\n{self.DIM}[Source: {source_ref}]{self.RESET}"
            if stream:
                for line in reply.split("\n"):
                    print(line)
                    time.sleep(0.01)
            else:
                print(reply)
            self.memory_bank.db.save_chat_message("user", user_query)
            self.memory_bank.db.save_chat_message("assistant", reply)
            return reply

        prompt = f"User: {user_query}\nAssistant:"
        prompt_tokens = self.tokenizer.encode(prompt, add_special_tokens=True)
        input_tensor = torch.tensor([prompt_tokens], dtype=torch.long, device=self.device)

        full_response = []
        token_stream = self.model.generate_stream(
            input_ids=input_tensor,
            max_new_tokens=max_tokens,
            temperature=temp,
            top_k=k,
            top_p=p,
            repetition_penalty=rep_pen,
            eos_token_id=self.tokenizer.eos_token_id
        )

        for token_id in token_stream:
            token_str = self.tokenizer.decode([token_id], skip_special_tokens=True)
            full_response.append(token_str)
            if stream:
                sys.stdout.write(token_str)
                sys.stdout.flush()

        if stream:
            sys.stdout.write("\n")

        final_reply = "".join(full_response).strip()
        if not final_reply:
            final_reply = "I don't have enough information on that topic in my memory. You can train me with -train -url <url> or -train -file <path>."
            if stream:
                print(final_reply)

        self.memory_bank.db.save_chat_message("user", user_query)
        self.memory_bank.db.save_chat_message("assistant", final_reply)
        return final_reply

    def start_interactive_session(self):
        chip = self.hw_info.get("chip_name", "Apple Silicon")
        device_type = self.device.type.upper()
        ram_mb = self.memory_bank.metadata.get("ram_size_mb", 0.0)
        num_chunks = self.memory_bank.metadata.get("total_chunks", 0)

        print("\n" + "=" * 70)
        print(f" {self.BOLD}{self.PURPLE}🤖 Tiwut-AI Interactive Neural Chat{self.RESET}")
        print(f" {self.DIM}⚡ Engine: {chip} [{device_type}] | In-RAM Memory: {ram_mb:.2f} MB ({num_chunks} chunks){self.RESET}")
        print(f" {self.DIM}💡 Commands: /help, /status, /memory, /clear, /exit{self.RESET}")
        print("=" * 70 + "\n")

        while True:
            try:
                user_input = input(f"{self.BOLD}{self.GREEN}You > {self.RESET}").strip()

                if not user_input:
                    continue

                if user_input.lower() in ["/exit", "/quit", "exit", "quit", ":q"]:
                    print(f"\n{self.CYAN}👋 Exiting Tiwut-AI. Have a great day!{self.RESET}\n")
                    break

                if user_input.lower() == "/help":
                    self._show_help()
                    continue

                if user_input.lower() == "/status":
                    self._show_status()
                    continue

                if user_input.lower() == "/memory":
                    self._show_memory()
                    continue

                if user_input.lower() == "/clear":
                    self.conversation_history.clear()
                    print(f"{self.YELLOW}🧹 Conversation history cleared.{self.RESET}\n")
                    continue

                sys.stdout.write(f"\n{self.BOLD}{self.CYAN}Tiwut-AI > {self.RESET}")
                sys.stdout.flush()

                t0 = time.time()
                response = self.generate_response(user_input, stream=True)
                elapsed = time.time() - t0

                self.conversation_history.append({"user": user_input, "bot": response})
                print(f"{self.DIM}[Inference: {elapsed:.3f}s | {self.device.type.upper()} Unified RAM]{self.RESET}\n")

            except (KeyboardInterrupt, EOFError):
                print(f"\n\n{self.CYAN}👋 Chat session ended.{self.RESET}\n")
                break

    def ask_single(self, question: str):
        print(f"\n{self.BOLD}{self.GREEN}Question:{self.RESET} {question}")
        sys.stdout.write(f"{self.BOLD}{self.CYAN}Answer:{self.RESET} ")
        sys.stdout.flush()

        t0 = time.time()
        response = self.generate_response(question, stream=True)
        elapsed = time.time() - t0

        print(f"{self.DIM}[Inference: {elapsed:.3f}s | {self.device.type.upper()} Unified RAM]{self.RESET}\n")

    def _show_help(self):
        print(f"\n{self.BOLD}Chat Commands:{self.RESET}")
        print("  /help    - Display this help message")
        print("  /status  - Show hardware, neural network parameters, and RAM usage")
        print("  /memory  - List learned sources and knowledge stored in RAM")
        print("  /clear   - Reset conversation context")
        print("  /exit    - Exit chat mode\n")

    def _show_status(self):
        total_params = sum(p.numel() for p in self.model.parameters())
        db_stats = self.memory_bank.db.get_stats()
        print(f"\n{self.BOLD}Neural Network & Hardware Status:{self.RESET}")
        print(f"  • Hardware:         {self.hw_info.get('chip_name')} ({self.hw_info.get('architecture')})")
        print(f"  • Compute Device:   {self.device.type.upper()} (MPS Metal Performance Shaders)")
        print(f"  • System RAM:       {self.hw_info.get('unified_ram_gb')} GB Unified Memory")
        print(f"  • Neural Model:     {total_params:,} parameters ({self.model.config.num_layers} layers, {self.model.config.num_heads} heads)")
        print(f"  • Vocab Size:       {self.tokenizer.vocab_size:,} tokens")
        print(f"  • High-Speed DB:    {db_stats['database_type']} ({db_stats['database_size_mb']} MB)")
        print(f"  • DB Location:      {db_stats['database_path']}")
        print(f"  • RAM Memory State: {self.memory_bank.metadata.get('ram_size_mb', 0)} MB resident")
        print(f"  • Stored Knowledge: {self.memory_bank.metadata.get('total_chunks', 0)} chunks / {self.memory_bank.metadata.get('total_tokens', 0):,} tokens\n")

    def _show_memory(self):
        sources = self.memory_bank.metadata.get("sources", [])
        total_chunks = self.memory_bank.metadata.get("total_chunks", 0)
        print(f"\n{self.BOLD}In-RAM Knowledge Base ({total_chunks} chunks):{self.RESET}")
        if not sources:
            print("  (No documents or websites trained yet. Use -train -url <url> or -train -file <path>)")
        else:
            for idx, src in enumerate(sources, 1):
                print(f"  {idx}. {src}")
