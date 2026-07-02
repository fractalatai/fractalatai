# Briefing: LoRA → GGUF Export Problem

## Context

We fine-tuned `unsloth/gemma-3-4b-it` with LoRA on a RunPod RTX 4090 for a 4-class legal text classification task (Hohfeldian position: active/counterparty/beneficiary/mentioned). Training used Unsloth + TRL SFTTrainer with 3,316 examples, 3 epochs, LoRA r=16.

## What succeeded

1. **Training**: 87 minutes on RTX 4090, completed without errors
2. **Adapter saved**: `adapter_model.safetensors` (125MB) + `adapter_config.json` — verified good, transferred to local machine
3. **Eval on RunPod** (with adapter loaded in Unsloth on GPU): **81.5% accuracy** on 583 held-out test examples, zero parse errors
4. **BF16 GGUF export**: `save_pretrained_gguf` produced a valid BF16 GGUF (7.3GB, valid GGUF header `47 47 55 46`)

## What failed

1. **Unsloth's Q4_K_M quantisation**: `save_pretrained_gguf(..., quantization_method="q4_k_m")` produced a file starting with all zeros (no GGUF magic bytes). Corrupt at source — not a transfer issue.

2. **Manual llama-quantize on /workspace**: `llama-quantize BF16.gguf output.gguf Q4_K_M` failed with `iostream error` when writing to RunPod's network-mounted `/workspace`. 

3. **Manual llama-quantize on /tmp**: Succeeded! Produced valid 2.4GB Q4_K_M GGUF with correct header. Transferred to local machine via `runpodctl`.

4. **Loading GGUF into Ollama locally**: Ollama loads the Q4_K_M GGUF fine. But **inference produces base-model-quality results** (~60% accuracy vs 81.5% on RunPod). The LoRA weights appear to NOT be merged into the GGUF.

## The core problem

The BF16 GGUF was exported by Unsloth's `save_pretrained_gguf`. This should:
1. Merge LoRA adapter weights into the base model
2. Export the merged model as BF16 GGUF
3. Quantise BF16 → Q4_K_M

Step 1 may not have worked correctly. The BF16 GGUF may be the base model WITHOUT the LoRA deltas applied. When we then quantise that to Q4_K_M, we get a valid GGUF file that behaves like the base model.

Evidence: the Q4_K_M GGUF scores ~60% locally (same as base gemma3:4b at 47.5% + system prompt improvement), while the adapter loaded on GPU via Unsloth scores 81.5%.

## Questions for Gemini

1. **How does Unsloth's `save_pretrained_gguf` merge LoRA weights?** Is there a known issue where the merge is skipped or silently fails?

2. **Is there a way to explicitly merge the LoRA adapter into the base model weights BEFORE exporting to GGUF?** For example:
   ```python
   model = model.merge_and_unload()  # PEFT method to merge LoRA into base
   model.save_pretrained("/workspace/merged_model")
   # Then convert merged_model to GGUF with llama.cpp's convert script
   ```

3. **Should we use llama.cpp's `convert_hf_to_gguf.py` on the merged HuggingFace model instead of Unsloth's GGUF export?**

4. **Any other approach to get a working GGUF from a LoRA adapter + base model?**

## Environment

- RunPod: RTX 4090, 24GB VRAM, 31GB RAM, PyTorch template
- Unsloth: latest (installed via `pip install unsloth`)
- Base model: `unsloth/gemma-3-4b-it` (4-bit quantised during training)
- Adapter: LoRA r=16, alpha=16, targeting q/k/v/o/gate/up/down projections
- Adapter config `base_model_name_or_path`: `unsloth/gemma-3-4b-it-unsloth-bnb-4bit`
- Local: Ollama for inference, Fedora Linux, CPU only
