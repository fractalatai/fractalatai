#!/usr/bin/env python3
"""Generate an image via the Gemini Interactions API.

Usage:
    generate.py --prompt "A flat icon of two people talking" --output icon.png
    generate.py --prompt "A blue speech bubble" --output icon.png --model gemini-3.1-flash-image
    generate.py --prompt "A poster layout" --output poster.png --size 2K --aspect 16:9
    generate.py --prompt "Edit: make background red" --output edited.png --input original.png

Models (cheapest to most expensive):
    gemini-3.1-flash-lite-image  — fastest, 1K only (default)
    gemini-3.1-flash-image       — versatile, up to 2K
    gemini-3-pro-image            — premium quality
"""
import argparse
import base64
import json
import os
import sys
import urllib.request
import urllib.error


API_URL = "https://generativelanguage.googleapis.com/v1beta/interactions"

MODELS = {
    "lite": "gemini-3.1-flash-lite-image",
    "flash": "gemini-3.1-flash-image",
    "pro": "gemini-3-pro-image",
}


def get_api_key():
    key = os.environ.get("GEMINI_API_KEY")
    if key:
        return key
    # Try sourcing from ~/.bashrc
    bashrc = os.path.expanduser("~/.bashrc")
    if os.path.exists(bashrc):
        with open(bashrc) as f:
            for line in f:
                if "GEMINI_API_KEY" in line and "export" in line:
                    # export GEMINI_API_KEY="..."
                    try:
                        key = line.split('"')[1]
                        return key
                    except IndexError:
                        pass
    print("ERROR: GEMINI_API_KEY not found in environment or ~/.bashrc", file=sys.stderr)
    sys.exit(1)


def generate_image(prompt, output_path, model="lite", input_path=None,
                   mime_type="image/png", size=None, aspect=None):
    api_key = get_api_key()
    model_name = MODELS.get(model, model)

    # Build input
    inputs = []
    if input_path:
        with open(input_path, "rb") as f:
            img_data = base64.b64encode(f.read()).decode()
        inputs.append({
            "type": "image",
            "mime_type": "image/png",
            "data": img_data,
        })
    inputs.append({"type": "text", "text": prompt})

    # Build request body
    body = {
        "model": model_name,
        "input": inputs,
    }

    # Optional response format
    response_format = {"type": "image", "mime_type": mime_type}
    if size:
        response_format["image_size"] = size
    if aspect:
        response_format["aspect_ratio"] = aspect
    if size or aspect:
        body["response_format"] = response_format

    # Make request
    data = json.dumps(body).encode()
    req = urllib.request.Request(
        API_URL,
        data=data,
        headers={
            "x-goog-api-key": api_key,
            "Content-Type": "application/json",
        },
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            result = json.loads(resp.read())
    except urllib.error.HTTPError as e:
        error_body = e.read().decode()
        print(f"ERROR: HTTP {e.code}", file=sys.stderr)
        try:
            err = json.loads(error_body)
            print(json.dumps(err.get("error", err), indent=2), file=sys.stderr)
        except json.JSONDecodeError:
            print(error_body[:500], file=sys.stderr)
        sys.exit(1)

    # Extract image from response
    image_saved = False
    for step in result.get("steps", []):
        for item in step.get("content", []):
            if item.get("type") == "image":
                img_bytes = base64.b64decode(item["data"])
                with open(output_path, "wb") as f:
                    f.write(img_bytes)
                image_saved = True
                break
        if image_saved:
            break

    if not image_saved:
        print("ERROR: No image found in response", file=sys.stderr)
        print(json.dumps(result, indent=2)[:1000], file=sys.stderr)
        sys.exit(1)

    # Report
    usage = result.get("usage", {})
    input_tokens = usage.get("total_input_tokens", "?")
    output_tokens = usage.get("total_output_tokens", "?")
    total_tokens = usage.get("total_tokens", "?")
    file_size = os.path.getsize(output_path)

    print(f"Saved: {output_path} ({file_size / 1024:.0f} KB)")
    print(f"Model: {model_name}")
    print(f"Tokens: {total_tokens} (input: {input_tokens}, output: {output_tokens})")


def main():
    parser = argparse.ArgumentParser(
        description="Generate images via Gemini Interactions API",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument("--prompt", required=True, help="Text prompt for image generation")
    parser.add_argument("--output", required=True, help="Output file path (PNG or JPEG)")
    parser.add_argument("--model", default="lite",
                        choices=["lite", "flash", "pro"],
                        help="Model tier: lite (cheapest), flash (versatile), pro (premium). Default: lite")
    parser.add_argument("--input", dest="input_path", default=None,
                        help="Input image for editing (optional)")
    parser.add_argument("--size", default=None, choices=["1K", "2K"],
                        help="Image size (only flash and pro support 2K)")
    parser.add_argument("--aspect", default=None,
                        help="Aspect ratio, e.g. 16:9, 1:1, 4:3")
    parser.add_argument("--mime", default="image/png",
                        choices=["image/png", "image/jpeg"],
                        help="Output MIME type. Default: image/png")

    args = parser.parse_args()
    generate_image(
        prompt=args.prompt,
        output_path=args.output,
        model=args.model,
        input_path=args.input_path,
        mime_type=args.mime,
        size=args.size,
        aspect=args.aspect,
    )


if __name__ == "__main__":
    main()
