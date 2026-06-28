!pip install unsloth -q

import os, json, torch, glob
from datasets import load_dataset
from collections import Counter
from unsloth import FastModel
from peft import PeftModel

TEST_FILE = "/kaggle/input/finetune-gemma3-position-jsonl/slm_test.jsonl"
test_dataset = load_dataset("json", data_files=TEST_FILE, split="train")

# Load base model
model, tokenizer = FastModel.from_pretrained(
    "unsloth/gemma-3-1b-it",
    max_seq_length=1024,
    load_in_4bit=True,
)

# Find and load the best checkpoint
checkpoints = sorted(glob.glob("/kaggle/working/gemma3-position-lora/checkpoint-*"))
print(f"Checkpoints found: {checkpoints}")
if checkpoints:
    model = PeftModel.from_pretrained(model, checkpoints[-1])
    print(f"Loaded LoRA from {checkpoints[-1]}")
elif os.path.exists("/kaggle/working/gemma3-position-lora/adapter_config.json"):
    model = PeftModel.from_pretrained(model, "/kaggle/working/gemma3-position-lora")
    print("Loaded LoRA from gemma3-position-lora/")
else:
    print("WARNING: No LoRA adapter found - evaluating base model only")

# Evaluate
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
