#!/usr/bin/env python3
"""
Asset Generation Script for Stroad Hero
Uses Google Gemini Nano Banana (gemini-nano-banana-2.1 / gemini-2.5-flash-image)
with pics/ideas/idea.jpg as the reference image to generate consistent pixel art assets.

Requirements:
    GEMINI_API_KEY environment variable set.
    Python 3.8+ (uses standard library only: urllib, base64, json, etc.)

Usage:
    export GEMINI_API_KEY="your-api-key"
    python3 scripts/generate_assets.py
    python3 scripts/generate_assets.py --category cars
    python3 scripts/generate_assets.py --asset car_red_pickup
"""

import argparse
import base64
import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

DEFAULT_MODEL = "gemini-nano-banana-2.1"
FALLBACK_MODELS = [
    "gemini-3.1-flash-image",
    "gemini-2.5-flash-image",
]

# Asset definitions tailored to the pixel art style of pics/ideas/idea.jpg
ASSETS = {
    # ---------------- Cars ----------------
    "car_red_pickup": {
        "category": "cars",
        "description": "Red heavy-duty pickup truck (American pickup truck style like the red truck in the reference image)",
        "prompt": (
            "An individual 2D video game sprite of a red American pickup truck (crew cab, full-size bed) driving away. "
            "Perspective: 3/4 rear-quarter isometric view matching the traffic perspective of the red pickup in the reference image. "
            "Style: high-quality 16-bit / 32-bit pixel art, crisp black outlines, stepped blocky lighting and shading, "
            "visible rear tail lights, chrome bumper, tailgate, side mirrors, and dark tinted windows. "
            "Isolated sprite centered on a pure solid chromakey magenta background (#FF00FF) with sharp edges. No ground, no shadows, no road."
        ),
    },
    "car_silver_sedan": {
        "category": "cars",
        "description": "Silver 4-door family sedan (Toyota/Honda style)",
        "prompt": (
            "An individual 2D video game sprite of a modern silver four-door sedan driving away. "
            "Perspective: 3/4 rear-quarter isometric view matching the road perspective of the silver car in the reference image. "
            "Style: high-quality 16-bit / 32-bit pixel art, crisp black outlines, subtle metallic shading, "
            "red tail lights, rear license plate, rear windshield. "
            "Isolated sprite centered on a pure solid chromakey magenta background (#FF00FF) with sharp edges. No ground, no shadows, no road."
        ),
    },
    "car_box_truck": {
        "category": "cars",
        "description": "White commercial box truck with yellow cab",
        "prompt": (
            "An individual 2D video game sprite of a commercial delivery box truck (yellow cab, tall white cargo box) driving away. "
            "Perspective: 3/4 rear-quarter isometric view matching the delivery trucks in the reference image. "
            "Style: high-quality 16-bit pixel art, black pixel outlines, clean stepped highlights and shadows, rollup rear door, mudflaps. "
            "Isolated sprite centered on a pure solid chromakey magenta background (#FF00FF) with sharp edges. No ground, no shadows, no road."
        ),
    },
    # ---------------- People ----------------
    "pedestrian_boy_hero": {
        "category": "people",
        "description": "Kid hero pedestrian (brown hair, gray shirt, jeans, sneakers)",
        "prompt": (
            "An individual 2D video game character sprite of the boy protagonist from the reference image. "
            "Appearance: messy brown hair swept to the side, short-sleeve neutral gray t-shirt, blue denim pants/jeans, brown and white sneakers. "
            "Pose: standing idle in side-profile / 3/4 view facing to the right, looking across the street. "
            "Style: crisp retro 16-bit / 32-bit pixel art, dark pixel outlines, expressive retro arcade proportions. "
            "Isolated character sprite centered on a pure solid chromakey magenta background (#FF00FF) with clean edges. No ground, no shadow."
        ),
    },
    "pedestrian_commuter": {
        "category": "people",
        "description": "Adult commuter pedestrian with coffee cup and bag",
        "prompt": (
            "An individual 2D video game character sprite of an adult commuter waiting on a sidewalk. "
            "Appearance: casual office wear, jacket, holding a takeaway coffee cup and carrying a shoulder messenger bag. "
            "Pose: standing in 3/4 perspective matching the human scale and angle of the boy in the reference image. "
            "Style: crisp 16-bit pixel art, bold dark contours, flat and stepped shading, matching the color palette and character proportions of the reference image. "
            "Isolated character sprite centered on a pure solid chromakey magenta background (#FF00FF) with clean edges. No ground, no shadow."
        ),
    },
    # ---------------- Buildings ----------------
    "building_strip_mall_pharmacy": {
        "category": "buildings",
        "description": "Suburban strip-mall pharmacy store ('Walgroins' / big-box pharmacy storefront)",
        "prompt": (
            "An individual 2D video game architectural asset of an American suburban strip mall pharmacy store building. "
            "Architecture: flat light-gray gravel roof with multiple boxy HVAC cooling units and vents, tan brick facade, "
            "prominent red bold signage across the upper frieze ('Walgroins' style), front glass display windows, green awning, automatic sliding glass doors. "
            "Perspective: semi-isometric 2.5D angle showing the front facade and flat rooftop, exactly matching the building angle on the right side of the reference image. "
            "Style: detailed 16-bit / 32-bit retro pixel art, crisp lines, commercial suburban stroad aesthetic. "
            "Isolated building asset on a pure solid chromakey magenta background (#FF00FF) with clean borders. No parking lot asphalt, no surrounding terrain."
        ),
    },
    "building_fast_food_joint": {
        "category": "buildings",
        "description": "Roadside fast food restaurant with tall pole marquee sign ('Taco Hell' style)",
        "prompt": (
            "An individual 2D video game architectural asset of a roadside fast-food restaurant building and its tall roadside pole sign. "
            "Architecture: flat-roofed commercial drive-thru building with purple/red accent band, drive-thru window, rooftop exhaust, "
            "and a tall roadside pole sign marquee ('Taco Hell' style bell logo on a pole). "
            "Perspective: 2.5D semi-isometric angle matching the street architecture in the reference image. "
            "Style: retro arcade pixel art, dark outlines, bold franchise colors, crisp edges. "
            "Isolated architectural asset centered on a pure solid chromakey magenta background (#FF00FF). No surrounding landscape, no ground."
        ),
    },
}


def load_reference_image(image_path: Path) -> tuple[str, str]:
    """Reads and encodes the reference image to base64, detecting MIME type from magic bytes."""
    if not image_path.exists():
        raise FileNotFoundError(f"Reference image not found: {image_path}")

    with open(image_path, "rb") as f:
        data = f.read()

    if data.startswith(b"\xff\xd8\xff"):
        mime_type = "image/jpeg"
    elif data.startswith(b"\x89PNG\r\n\x1a\n"):
        mime_type = "image/png"
    elif data.startswith(b"RIFF") and len(data) >= 12 and data[8:12] == b"WEBP":
        mime_type = "image/webp"
    elif data.startswith((b"GIF87a", b"GIF89a")):
        mime_type = "image/gif"
    else:
        ext = image_path.suffix.lower()
        mime_type = "image/png" if ext == ".png" else "image/jpeg"

    encoded = base64.b64encode(data).decode("utf-8")
    return encoded, mime_type


def call_interactions_api(
    api_key: str, model: str, prompt: str, ref_b64: str, mime_type: str
) -> bytes:
    """Calls Gemini Interactions API (Nano Banana)."""
    url = f"https://generativelanguage.googleapis.com/v1beta/interactions?key={api_key}"

    full_prompt = (
        f"Reference image provides the exact visual style: 16-bit / 32-bit retro pixel art, "
        f"color palette, line thickness, perspective, and stroad-themed video game aesthetic.\n"
        f"Task: Generate an isolated asset matching this exact style.\n\n"
        f"Asset specification: {prompt}"
    )

    payload = {
        "model": model,
        "input": [
            {
                "type": "text",
                "text": full_prompt,
            },
            {
                "type": "image",
                "mime_type": mime_type,
                "data": ref_b64,
            },
        ],
    }

    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "x-goog-api-key": api_key,
        },
        method="POST",
    )

    with urllib.request.urlopen(req, timeout=120) as resp:
        result = json.loads(resp.read().decode("utf-8"))
    return extract_image_from_response(result)


def extract_image_from_response(result: dict) -> bytes:
    """Extracts raw image bytes from Interactions API or generateContent response structures."""
    # 1. Check steps (Interactions API structure: result['steps'][...]['content'][...])
    steps = result.get("steps")
    if isinstance(steps, list):
        for step in reversed(steps):
            content = step.get("content", [])
            items = content if isinstance(content, list) else [content]
            for item in items:
                if not isinstance(item, dict):
                    continue
                if item.get("type") == "image" and "data" in item:
                    return base64.b64decode(item["data"])
                if "data" in item and str(item.get("mime_type", "")).startswith("image/"):
                    return base64.b64decode(item["data"])
                if "image" in item and isinstance(item["image"], dict) and "data" in item["image"]:
                    return base64.b64decode(item["image"]["data"])
                if "inline_data" in item and isinstance(item["inline_data"], dict) and "data" in item["inline_data"]:
                    return base64.b64decode(item["inline_data"]["data"])
                if "inlineData" in item and isinstance(item["inlineData"], dict) and "data" in item["inlineData"]:
                    return base64.b64decode(item["inlineData"]["data"])

    # 2. Check output_image or interaction.output_image
    output_image = result.get("output_image")
    if not output_image and "interaction" in result and isinstance(result["interaction"], dict):
        output_image = result["interaction"].get("output_image")
    if isinstance(output_image, dict) and "data" in output_image:
        return base64.b64decode(output_image["data"])

    # 3. Check candidates (standard generateContent response structure)
    candidates = result.get("candidates")
    if isinstance(candidates, list) and candidates:
        for cand in candidates:
            parts = cand.get("content", {}).get("parts", [])
            for part in parts:
                if isinstance(part, dict):
                    if "inline_data" in part and "data" in part["inline_data"]:
                        return base64.b64decode(part["inline_data"]["data"])
                    if "inlineData" in part and "data" in part["inlineData"]:
                        return base64.b64decode(part["inlineData"]["data"])

    # 4. Check outputs list
    outputs = result.get("outputs")
    if isinstance(outputs, list):
        for out in outputs:
            if isinstance(out, dict):
                if out.get("type") == "image" and "data" in out:
                    return base64.b64decode(out["data"])
                if "data" in out and str(out.get("mime_type", "")).startswith("image/"):
                    return base64.b64decode(out["data"])

    available_keys = list(result.keys())
    raise ValueError(
        f"No image data returned from API. Top-level keys: {available_keys}. "
        f"Response snippet: {json.dumps(result)[:500]}"
    )


def call_generate_content_api(
    api_key: str, model: str, prompt: str, ref_b64: str, mime_type: str
) -> bytes:
    """Fallback using standard models:generateContent endpoint."""
    url = f"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"

    full_prompt = (
        f"Style reference: maintain the pixel art style, palette, and perspective of the attached image.\n"
        f"Generate this isolated asset sprite on a pure white background: {prompt}"
    )

    payload = {
        "contents": [
            {
                "parts": [
                    {
                        "inline_data": {
                            "mime_type": mime_type,
                            "data": ref_b64,
                        }
                    },
                    {
                        "text": full_prompt,
                    },
                ]
            }
        ],
        "generationConfig": {
            "responseModalities": ["IMAGE"],
        },
    }

    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "x-goog-api-key": api_key,
        },
        method="POST",
    )

    with urllib.request.urlopen(req, timeout=120) as resp:
        result = json.loads(resp.read().decode("utf-8"))
    return extract_image_from_response(result)


def generate_asset(
    api_key: str,
    model: str,
    asset_key: str,
    ref_b64: str,
    mime_type: str,
    out_dir: Path,
) -> Path:
    """Generates a single asset with fallback handling."""
    asset_info = ASSETS[asset_key]
    category = asset_info["category"]
    prompt = asset_info["prompt"]

    target_dir = out_dir / category
    target_dir.mkdir(parents=True, exist_ok=True)
    out_file = target_dir / f"{asset_key}.png"

    print(f"\n🎨 Generating [{category}] {asset_key}...")
    print(f"   Prompt: {asset_info['description']}")

    models_to_try = [model] + [m for m in FALLBACK_MODELS if m != model]
    last_err = None

    for candidate_model in models_to_try:
        try:
            print(f"   Trying model: {candidate_model} (Interactions API)...")
            img_bytes = call_interactions_api(
                api_key, candidate_model, prompt, ref_b64, mime_type
            )
            with open(out_file, "wb") as f:
                f.write(img_bytes)
            print(f"   ✅ Saved: {out_file} ({len(img_bytes)} bytes)")
            return out_file
        except (urllib.error.URLError, OSError, ValueError, KeyError, TypeError, json.JSONDecodeError, RuntimeError) as e:
            if isinstance(e, urllib.error.HTTPError):
                err_body = e.read().decode("utf-8", errors="replace")
                print(f"   ⚠️ HTTP {e.code} with {candidate_model} (Interactions API): {err_body[:200]}")
            else:
                print(f"   ⚠️ Error with {candidate_model} (Interactions API): {e}")
            last_err = e

            # Try generateContent fallback
            try:
                print(f"   Trying model: {candidate_model} (generateContent API)...")
                img_bytes = call_generate_content_api(
                    api_key, candidate_model, prompt, ref_b64, mime_type
                )
                with open(out_file, "wb") as f:
                    f.write(img_bytes)
                print(f"   ✅ Saved: {out_file} ({len(img_bytes)} bytes)")
                return out_file
            except (urllib.error.URLError, OSError, ValueError, KeyError, TypeError, json.JSONDecodeError, RuntimeError) as e2:
                print(f"   ⚠️ Error with {candidate_model} (generateContent API): {e2}")
                last_err = e2
                continue

    raise RuntimeError(f"Failed to generate {asset_key}. Last error: {last_err}")


def main():
    parser = argparse.ArgumentParser(
        description="Generate individual pixel art game assets using Gemini Nano Banana."
    )
    parser.add_argument(
        "--ref", default="pics/ideas/idea.jpg", help="Path to reference style image"
    )
    parser.add_argument(
        "--output-dir",
        default="pics/assets",
        help="Directory to save generated asset sprites",
    )
    parser.add_argument(
        "--category",
        choices=["cars", "people", "buildings", "all"],
        default="all",
        help="Category to generate",
    )
    parser.add_argument(
        "--asset", help="Specific asset name to generate (e.g. car_red_pickup)"
    )
    parser.add_argument(
        "--model",
        default=DEFAULT_MODEL,
        help=f"Gemini Nano Banana model (default: {DEFAULT_MODEL})",
    )
    args = parser.parse_args()

    api_key = os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
    if not api_key:
        print("=" * 70, file=sys.stderr)
        print(
            "ERROR: GEMINI_API_KEY or GOOGLE_API_KEY environment variable is not set.",
            file=sys.stderr,
        )
        print("=" * 70, file=sys.stderr)
        print(
            "To generate the assets using Gemini Nano Banana, export your API key:",
            file=sys.stderr,
        )
        print('  export GEMINI_API_KEY="AIzaSy..."', file=sys.stderr)
        print(f"  python3 {sys.argv[0]}", file=sys.stderr)
        print("=" * 70, file=sys.stderr)
        sys.exit(1)

    repo_root = Path(__file__).resolve().parent.parent
    ref_path = (
        (repo_root / args.ref) if not Path(args.ref).is_absolute() else Path(args.ref)
    )
    out_dir = (
        (repo_root / args.output_dir)
        if not Path(args.output_dir).is_absolute()
        else Path(args.output_dir)
    )

    print(f"Loading reference image from: {ref_path}")
    ref_b64, mime_type = load_reference_image(ref_path)

    # Determine which assets to process
    if args.asset:
        if args.asset not in ASSETS:
            print(
                f"Unknown asset: {args.asset}. Available: {list(ASSETS.keys())}",
                file=sys.stderr,
            )
            sys.exit(1)
        targets = [args.asset]
    elif args.category != "all":
        targets = [k for k, v in ASSETS.items() if v["category"] == args.category]
    else:
        targets = list(ASSETS.keys())

    print(f"Found {len(targets)} asset(s) to generate in category '{args.category}':")
    for t in targets:
        print(f"  - [{ASSETS[t]['category']}] {t}: {ASSETS[t]['description']}")

    success_count = 0
    for target in targets:
        try:
            generate_asset(api_key, args.model, target, ref_b64, mime_type, out_dir)
            success_count += 1
            # Brief delay between API calls to avoid rate limit spikes
            time.sleep(1.0)
        except (urllib.error.URLError, OSError, ValueError, KeyError, TypeError, json.JSONDecodeError, RuntimeError) as e:
            print(f"❌ Failed to generate {target}: {e}", file=sys.stderr)

    print(
        f"\nDone! Successfully generated {success_count}/{len(targets)} assets in '{out_dir}'."
    )


if __name__ == "__main__":
    main()
