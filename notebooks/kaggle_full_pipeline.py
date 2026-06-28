# === CELL 1: Install ===
# !pip install unsloth -q

# === CELL 2: Full pipeline — train + save + eval + export ===
# Run this as ONE cell after install. Do not restart kernel.

import os, json, torch, glob
from datasets import load_dataset
from collections import Counter
from unsloth import FastModel
from trl import SFTTrainer, SFTConfig

# --- Config ---
DATA_DIR = "/kaggle/input/datasets/shotleybuilder/finetune-gemma3-position-jsonl"
TRAIN_FILE = os.path.join(DATA_DIR, "slm_train.jsonl")
TEST_FILE = os.path.join(DATA_DIR, "slm_test.jsonl")
OUTPUT_DIR = "/kaggle/working/gemma3-position-lora"
GGUF_DIR = "/kaggle/working/gemma3-position-gguf"

# --- 1. Load model ---
print("=== Loading model ===")
model, tokenizer = FastModel.from_pretrained(
    "unsloth/gemma-3-1b-it",
    max_seq_length=1024,
    load_in_4bit=True,
)

# --- 2. Configure LoRA ---
print("=== Configuring LoRA ===")
model = FastModel.get_peft_model(
    model,
    r=16,
    lora_alpha=16,
    lora_dropout=0,
    target_modules=[
        "q_proj", "k_proj", "v_proj", "o_proj",
        "gate_proj", "up_proj", "down_proj",
    ],
    use_gradient_checkpointing="unsloth",
)

trainable = sum(p.numel() for p in model.parameters() if p.requires_grad)
total_params = sum(p.numel() for p in model.parameters())
print(f"Trainable: {trainable:,} / {total_params:,} ({100*trainable/total_params:.1f}%)")

# --- 3. Load data ---
print("=== Loading data ===")
train_dataset = load_dataset("json", data_files=TRAIN_FILE, split="train")
test_dataset = load_dataset("json", data_files=TEST_FILE, split="train")
print(f"Train: {len(train_dataset)}, Test: {len(test_dataset)}")

def apply_chat_template(examples):
    texts = tokenizer.apply_chat_template(examples["messages"], tokenize=False)
    return {"text": texts}

train_dataset = train_dataset.map(apply_chat_template, batched=True)
test_dataset_mapped = test_dataset.map(apply_chat_template, batched=True)

# --- 4. Train ---
print("=== Training ===")
trainer = SFTTrainer(
    model=model,
    tokenizer=tokenizer,
    train_dataset=train_dataset,
    eval_dataset=test_dataset_mapped,
    args=SFTConfig(
        output_dir=OUTPUT_DIR,
        num_train_epochs=3,
        per_device_train_batch_size=2,
        gradient_accumulation_steps=4,
        learning_rate=2e-4,
        lr_scheduler_type="cosine",
        warmup_ratio=0.1,
        logging_steps=25,
        eval_strategy="steps",
        eval_steps=100,
        save_strategy="steps",
        save_steps=200,
        fp16=True,
        optim="adamw_8bit",
        seed=42,
        report_to="none",
        dataset_text_field="text",
        max_seq_length=1024,
    ),
)
trainer.train()

# --- 5. Save adapter ---
print("=== Saving LoRA adapter ===")
model.save_pretrained(OUTPUT_DIR)
tokenizer.save_pretrained(OUTPUT_DIR)
print(f"Saved to {OUTPUT_DIR}")

# --- 6. Evaluate ---
print("=== Evaluating on test set ===")
correct = 0
total = 0
errors = 0
confusion = Counter()

for i, example in enumerate(test_dataset):
    messages = example["messages"]
    eval_messages = [m for m in messages if m["role"] != "assistant"]
    gold = json.loads(messages[-1]["content"])["position"]

    input_text = tokenizer.apply_chat_template(eval_messages, tokenize=False, add_generation_prompt=True)
    inputs = tokenizer(input_text, return_tensors="pt").to(model.device)

    with torch.no_grad():
        outputs = model.generate(**inputs, max_new_tokens=50, temperature=0.0, do_sample=False)

    response = tokenizer.decode(outputs[0][inputs["input_ids"].shape[1]:], skip_special_tokens=True).strip()

    try:
        if "```json" in response:
            response = response.split("```json")[1].split("```")[0].strip()
        elif "```" in response:
            response = response.split("```")[1].split("```")[0].strip()
        predicted = json.loads(response)["position"]
        if predicted in ("active", "counterparty", "beneficiary", "mentioned"):
            total += 1
            if predicted == gold:
                correct += 1
            confusion[(predicted, gold)] += 1
        else:
            errors += 1
    except (json.JSONDecodeError, KeyError):
        errors += 1

    if (i + 1) % 50 == 0:
        acc = 100 * correct / total if total > 0 else 0
        print(f"  [{i+1}/{len(test_dataset)}] {correct}/{total} = {acc:.1f}%")

acc = 100 * correct / total if total > 0 else 0
print(f"\nTest accuracy: {correct}/{total} = {acc:.1f}%")
print(f"Parse errors: {errors}")
print(f"Baseline (prompt-only gemma3:4b): 47.5%")
positions = ["active", "counterparty", "beneficiary", "mentioned"]
print(f"\nConfusion (predicted -> gold):")
print(f"  {'':15s} " + " ".join(f"{p:>12s}" for p in positions))
for pred in positions:
    counts = [confusion.get((pred, gold), 0) for gold in positions]
    print(f"  {pred:15s} " + " ".join(f"{c:12d}" for c in counts))

# --- 7. Export GGUF ---
print("\n=== Exporting GGUF ===")
model.save_pretrained_gguf(GGUF_DIR, tokenizer, quantization_method="q4_k_m")
gguf_files = glob.glob(f"{GGUF_DIR}/*.gguf")
print(f"GGUF files: {gguf_files}")
for f in gguf_files:
    size_mb = os.path.getsize(f) / 1024 / 1024
    print(f"  {f} ({size_mb:.0f} MB)")
print("\nDone! Download GGUF from /kaggle/working/gemma3-position-gguf/")
