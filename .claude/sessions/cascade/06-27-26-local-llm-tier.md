---
session: Local SLM Tier
status: closed
opened: 2026-06-27
closed: 2026-06-28
outcome: success

summary: >
  Fine-tuned gemma-3-4b-it with LoRA (16-bit) on RunPod RTX 5090 for
  Hohfeldian position classification. 82.1% test accuracy on RunPod, GGUF
  exported and loaded into Ollama locally. Full local eval pending.

decisions:
  - what: Fine-tune gemma-3-4b-it with LoRA on RunPod
    why: Base model scored 47.5% — doesn't understand Hohfeldian taxonomy without domain training
    result: 82.1% accuracy on 583 held-out test examples, zero parse errors

  - what: 16-bit training (not 4-bit) for clean GGUF export
    why: 4-bit LoRA merge loses weights during quantisation — 81.5% on GPU drops to 50% in GGUF
    result: 16-bit merge preserves LoRA deltas, clean GGUF loaded into Ollama

  - what: Use unsloth/gemma-3-4b-it (ungated) via Unsloth + TRL SFTTrainer
    why: Google's HuggingFace Gemma repo requires account + licence acceptance
    result: No sign-up, downloads immediately on RunPod

metrics:
  finetuned_4b_16bit: { accuracy: 82.1%, training_time: 90min, gpu: RTX_5090, cost: ~$2 }
  prompt_engineering_baseline: { accuracy: 47.5%, active: 85.1%, counterparty: 42.5%, beneficiary: 0.0%, mentioned: 12.1% }
  training_data: { train: 3316, test: 583, classes: 4 }

lessons:
  - title: 16-bit training required for working GGUF
    detail: 4-bit QLoRA works on GPU but LoRA weights get rounded away during GGUF quantisation. Always use load_in_4bit=False if the goal is an Ollama-deployable GGUF.
    tag: models

  - title: RunPod SSH key injection for existing pods
    detail: Keys added in RunPod Settings only apply to new pods. For existing pods, inject via Jupyter terminal — echo "$(cat ~/.ssh/id_ed25519.pub)" >> /root/.ssh/authorized_keys
    tag: infrastructure

  - title: Use runpodctl for large file transfers
    detail: Jupyter browser downloads corrupt files >500MB. runpodctl peer-to-peer is reliable. Install locally from GitHub releases.
    tag: infrastructure

  - title: Write GGUF files to /tmp on RunPod, not /workspace
    detail: Network-mounted /workspace causes iostream errors during quantisation. /tmp uses container-local disk.
    tag: infrastructure

  - title: Verify GGUF header before transferring
    detail: Unsloth export can produce corrupt files silently. Check od -A x -t x1z -N 16 — must show 47 47 55 46 (GGUF magic).
    tag: models

  - title: Use /api/chat not /api/generate for fine-tuned Ollama models
    detail: Model trained with chat template (system/user/assistant). Raw prompt doesn't match training format.
    tag: models

  - title: Add --break-system-packages to pip on some RunPod templates
    detail: Some templates use externally-managed Python. Add flag in both shell and subprocess calls.
    tag: infrastructure

  - title: Eval base model first to diagnose what fine-tuning must fix
    detail: Per-class breakdown (beneficiary=0%, mentioned=12%) showed exactly where the model failed. Fine-tuning fixed both to 70%+.
    tag: methodology

artifacts:
  - scripts/finetune_runpod_16bit.py
  - scripts/eval_slm_position.py
  - scripts/export_slm_training_data.py
  - data/gemma3-position-q4.gguf
  - data/slm-adapter/adapter_model.safetensors
  - data/slm_train.jsonl
  - data/slm_test.jsonl
  - data/Modelfile
  - .claude/commands/session-close.md
  - .claude/skills/runpod-finetune/SKILL.md

depends_on:
  - 06-26-26-reconciliation
  - 06-26-26-benchmark-qa

enables:
  - Full local eval of fine-tuned SLM
  - Pipeline wiring — SLM predictions to provision_actors
  - Reconciliation with LLM tier populated
---

# Session: Local SLM Tier — gemma3:4b via Ollama (CLOSED)

## Problem

The reconciliation engine flags 34.6% of actors as `pending_llm` (3,735 of 10,795 across benchmarks). These are cases where regex and classifier disagree and the classifier isn't confident (< 0.7). They sit at 37.3% accuracy — effectively a coin flip. A local SLM can resolve these without cloud API costs.

The pipeline cascade:
```
Regex (Tier 1) → Classifier (Tier 2) → Local SLM (Tier 3) → Gemini (Tier 4, paid)
```

## Success path

1. ✅ Prepare training data: `scripts/export_slm_training_data.py` → 3,316 train / 583 test (stratified, HuggingFace chat format)
2. ✅ Evaluate base model (prompt engineering): 47.5% — massive active bias, 0% beneficiary
3. ✅ Fine-tune gemma-3-4b-it with LoRA on RunPod RTX 4090 (~87 min, ~$1)
4. ✅ Eval on RunPod: **81.5%** (active 88.9%, counterparty 72.3%, beneficiary 71.7%, mentioned 82.9%)
5. ✅ LoRA adapter saved: `data/slm-adapter/adapter_model.safetensors` (125MB)
6. ✅ Re-export GGUF with 16-bit training + proper LoRA merge on RunPod RTX 5090
7. ✅ Load valid GGUF into Ollama as `gemma3-position`
8. ⏸️ Eval locally against pending_llm benchmark actors (deferred — next session)
9. ⏸️ Wire into pipeline (deferred — next session)

## RunPod procedure (for re-export)

### Setup
- RTX 4090 (24GB VRAM), PyTorch template, 30GB+ volume disk
- Upload: `scripts/finetune_runpod.py`, `data/slm_train.jsonl`, `data/slm_test.jsonl`
- Install: `pip install unsloth -q`

### Fix SSH on existing pods
```bash
# On pod via Jupyter terminal:
echo "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIP86op1UQJcFjSucDWG7JONDfj4cx06Q56k1NbjR3KL8 jason.woodruff@hey.com" >> /root/.ssh/authorized_keys

# Then from local machine (check Connect tab for current port):
scp -P <PORT> -i ~/.ssh/id_ed25519 -o StrictHostKeyChecking=no root@<IP>:/workspace/output/file ./
```

### GGUF export fix
```bash
# Write to /tmp (not /workspace — network mount causes iostream errors)
# Use llama-quantize directly (Unsloth's wrapper produces corrupt Q4_K_M)
/root/.unsloth/llama.cpp/llama-quantize /path/to/BF16.gguf /tmp/gemma3-position-q4.gguf Q4_K_M

# Verify header before transferring
od -A x -t x1z -N 16 /tmp/gemma3-position-q4.gguf
# Must show: 47 47 55 46 (GGUF magic)

# Transfer with runpodctl (not Jupyter download — browser corrupts 2GB+ files)
runpodctl send /tmp/gemma3-position-q4.gguf
# On local machine:
/tmp/runpodctl receive <CODE>
```

### Load into Ollama
```bash
cd /var/home/jason/fractalaw/data
ollama create gemma3-position -f Modelfile
# Modelfile: FROM ./gemma3-position-q4.gguf + temperature 0.0 + num_predict 50
```

## Remaining work

1. ✅ Re-export GGUF on RunPod with 16-bit training + proper LoRA merge (82.1% on RunPod)
2. ✅ Load into Ollama locally (gemma3-position, 2.4GB Q4_K_M)
3. ⏸️ Full eval locally — confirm ~82% (deferred — next session)
4. ⏸️ Wire into pipeline: write SLM predictions to llm_drrp/llm_position (deferred — next session)
5. ⏸️ Re-run taxa reconcile + backfill (deferred — next session)
6. ⏸️ Benchmark overall reconciled accuracy improvement (deferred — next session)

## Dependencies

- ✅ Reconciliation engine complete (pending_llm actors identified)
- ✅ provision_actors with per-tier columns (llm_drrp, llm_position ready)
- ✅ Gold benchmarks loaded (4,062 actors)
- ✅ Ollama running
- ✅ Reconciliation rules already handle LLM tier (LLM wins)
- ✅ Stage 1 eval complete — baseline at 47.5%
- ✅ LoRA adapter trained and saved (81.5% on RunPod)
- ✅ runpodctl installed locally at /tmp/runpodctl
