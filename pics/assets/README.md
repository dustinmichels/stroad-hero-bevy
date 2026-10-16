# Stroad Hero Sprite Assets

Generated using Google Gemini Nano Banana (`gemini-nano-banana-2.1`) using `pics/ideas/idea.jpg` as the style reference.

## Directory Structure

- `cars/`: Individual vehicle sprites (rear/isometric 3/4 perspective matching stroad traffic flow).
  - `car_red_pickup.png`
  - `car_silver_sedan.png`
  - `car_box_truck.png`
- `people/`: Pedestrian sprites (isometric/profile proportions).
  - `pedestrian_boy_hero.png`
  - `pedestrian_commuter.png`
- `buildings/`: Commercial stroad architecture (strip mall pharmacy, fast-food joint, roadside marquee signs).
  - `building_strip_mall_pharmacy.png`
  - `building_fast_food_joint.png`

## Generating / Updating Assets

Set your Gemini API key:

```bash
export GEMINI_API_KEY="your-api-key"
```

Run the generator:

```bash
# Generate all asset categories
python3 scripts/generate_assets.py

# Or generate a specific category
python3 scripts/generate_assets.py --category cars
python3 scripts/generate_assets.py --category people
python3 scripts/generate_assets.py --category buildings

# Or generate a single asset
python3 scripts/generate_assets.py --asset car_red_pickup
```
