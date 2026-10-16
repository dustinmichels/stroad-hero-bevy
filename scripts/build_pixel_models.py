"""Build 3D .blend variants of the pixel-art sprites in pics/assets.

Run: blender -b -P scripts/build_pixel_models.py

Writes art/models/<category>/<asset>_<variant>.blend and art/models/previews/*.png.
Units: 1 Blender unit = 1 game unit (car 48x26, player 30 tall). Cars face +X,
origin at bottom center, so glTF export lands on Bevy's heading-0 = +X.

Variants:
  cars/buildings: lowpoly (flat-colour boxes + inverted-hull outline)
                  voxel   (same parts snapped to a voxel grid + outline)
  people:         billboard (sprite on a quad, nearest sampling, alpha clip)
                  extruded  (each sprite pixel as a thin prism)
"""

import math
from pathlib import Path

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "pics" / "assets"
OUT = ROOT / "art" / "models"
PREVIEWS = OUT / "previews"

OUTLINE_HEX = "1a1420"
PITCH_DEG, YAW_DEG = 30.0, 45.0  # docs/isometric-view.md §3.3 (2:1 pixel iso)
INGAME_UNITS_PER_PX = 2.5  # 320px render target upscaled to the 800-unit window

# Parts: ("box", lo, hi, colour) | ("wheel", centre, radius, width, colour) (axis Y)
#        | ("cylz", base_centre, radius, height, colour) (axis Z).
# Later parts win in the voxel variant, so details (glass, lights) come after bodies.
# Car coords are even so voxel (v=2) centres never sit on a boundary; buildings use v=4.

ASSETS = {
    "car_silver_sedan": {
        "category": "cars", "voxel": 2.0, "outline": 0.8, "detail_upp": 0.5,
        "palette": {"body": "b5bcc6", "glass": "2c3a48", "tire": "222428", "hub": "8a929c",
                     "tail": "d02a20", "head": "f2e2a0", "plate": "dfe6f0"},
        "parts": [
            ("box", (-24, -13, 4), (24, 13, 10), "body"),
            ("box", (-12, -11, 10), (12, 11, 18), "body"),
            ("box", (-12.4, -11.4, 12), (12.4, 11.4, 16.4), "glass"),
            ("wheel", (-14, -10.5, 5), 5, 5, "tire"),
            ("wheel", (14, -10.5, 5), 5, 5, "tire"),
            ("wheel", (-14, 10.5, 5), 5, 5, "tire"),
            ("wheel", (14, 10.5, 5), 5, 5, "tire"),
            ("wheel", (-14, -10.5, 5), 2.5, 5.6, "hub"),
            ("wheel", (14, -10.5, 5), 2.5, 5.6, "hub"),
            ("box", (-24.4, -12, 6), (-22, -8, 10), "tail"),
            ("box", (-24.4, 8, 6), (-22, 12, 10), "tail"),
            ("box", (-24.4, -3, 4), (-22, 3, 8), "plate"),
            ("box", (22, -12, 6), (24.4, -8, 10), "head"),
            ("box", (22, 8, 6), (24.4, 12, 10), "head"),
        ],
    },
    "car_red_pickup": {
        "category": "cars", "voxel": 2.0, "outline": 0.8, "detail_upp": 0.5,
        "palette": {"body": "c0201c", "bed": "6e1410", "glass": "2a3540", "tire": "222428",
                     "chrome": "b8bcc0", "tail": "ff4030", "head": "f2e2a0"},
        "parts": [
            ("box", (-28, -14, 4), (28, 14, 12), "body"),
            ("box", (-4, -12, 12), (14, 12, 22), "body"),
            ("box", (-26, -12, 6), (-6, 12, 12.4), "bed"),
            ("box", (-4.4, -12.4, 14), (14.4, 12.4, 20.4), "glass"),
            ("wheel", (-17, -11.5, 6), 6, 5, "tire"),
            ("wheel", (17, -11.5, 6), 6, 5, "tire"),
            ("wheel", (-17, 11.5, 6), 6, 5, "tire"),
            ("wheel", (17, 11.5, 6), 6, 5, "tire"),
            ("wheel", (-17, -11.5, 6), 3, 5.6, "chrome"),
            ("wheel", (17, -11.5, 6), 3, 5.6, "chrome"),
            ("box", (-30, -14, 2), (-28, 14, 6), "chrome"),
            ("box", (28, -14, 2), (30, 14, 6), "chrome"),
            ("box", (-28.4, -14, 8), (-26, -10, 12), "tail"),
            ("box", (-28.4, 10, 8), (-26, 14, 12), "tail"),
            ("box", (26, -13, 8), (28.4, -9, 12), "head"),
            ("box", (26, 9, 8), (28.4, 13, 12), "head"),
        ],
    },
    "car_box_truck": {
        "category": "cars", "voxel": 2.0, "outline": 0.8, "detail_upp": 0.6,
        "palette": {"cab": "f2b020", "box": "e8ecf0", "door": "9aa0a8", "chassis": "303438",
                     "glass": "2a3540", "tire": "222428", "hub": "9aa0a8", "tail": "e84020"},
        "parts": [
            ("box", (-38, -12, 4), (38, 12, 8), "chassis"),
            ("box", (22, -14, 6), (38, 14, 26), "cab"),
            ("box", (26, -14.4, 16), (38.4, 14.4, 24), "glass"),
            ("box", (-38, -16, 8), (20, 16, 38), "box"),
            ("box", (-38.4, -14, 10), (-36, 14, 36), "door"),
            ("box", (-40, -14, 2), (-38, 14, 4), "chassis"),
            ("box", (-38.4, -16, 10), (-36, -14, 16), "tail"),
            ("box", (-38.4, 14, 10), (-36, 16, 16), "tail"),
            ("wheel", (-22, -11, 6), 6, 6, "tire"),
            ("wheel", (30, -11, 6), 6, 6, "tire"),
            ("wheel", (-22, 11, 6), 6, 6, "tire"),
            ("wheel", (30, 11, 6), 6, 6, "tire"),
            ("wheel", (-22, -11, 6), 3, 6.6, "hub"),
            ("wheel", (30, -11, 6), 3, 6.6, "hub"),
        ],
    },
    "building_fast_food_joint": {
        "category": "buildings", "voxel": 4.0, "outline": 1.5, "detail_upp": 1.2,
        "palette": {"wall": "d9c6a5", "stone": "8c8478", "purple": "5a3a8c", "red": "c0282a",
                     "roof": "8e8e96", "glass": "5a86b0", "unit": "a8b0b8", "pole": "2a2a35",
                     "cream": "f2ebd0"},
        "parts": [
            ("box", (-68, -52, 0), (68, 52, 32), "wall"),
            ("box", (-68.4, -52.4, 0), (68.4, 52.4, 8), "stone"),
            ("box", (-68.4, -52.4, 20), (68.4, 52.4, 24), "red"),
            ("box", (-68.4, -52.4, 24), (68.4, 52.4, 28), "purple"),
            ("box", (-68, -52, 32), (68, 52, 36), "purple"),
            ("box", (-64, -48, 33), (64, 48, 36.4), "roof"),
            ("box", (0, -52.4, 8), (56, -49, 20), "glass"),
            ("box", (-68.4, -40, 8), (-65, 40, 20), "glass"),
            ("box", (-40, -56, 0), (-12, -48, 44), "wall"),
            ("box", (-40.4, -56.4, 0), (-11.6, -47.6, 8), "stone"),
            ("box", (-40.4, -56.4, 32), (-11.6, -47.6, 40), "purple"),
            ("box", (-34, -56.4, 8), (-18, -53, 24), "glass"),
            ("box", (36, -56, 0), (60, -48, 40), "red"),
            ("box", (20, 8, 36), (36, 24, 44), "unit"),
            ("box", (-28, -12, 36), (-12, 4, 44), "unit"),
            ("cylz", (-48, 24, 36), 4, 8, "unit"),
            ("cylz", (-32, 32, 36), 4, 8, "unit"),
            ("cylz", (-96, -72, 0), 2.5, 64, "pole"),
            ("box", (-112, -74, 60), (-80, -70, 96), "purple"),
            ("box", (-108, -74.4, 64), (-84, -69.6, 92), "cream"),
        ],
    },
    "building_strip_mall_pharmacy": {
        "category": "buildings", "voxel": 4.0, "outline": 1.5, "detail_upp": 1.5,
        "palette": {"walk": "b0aaa0", "wall": "d2a878", "brick": "9a4a32", "red": "d02828",
                     "cap": "e2c69a", "roof": "9a9aa0", "glass": "7aa0c0", "awning": "2e6a48",
                     "unit": "a0a8b0"},
        "parts": [
            ("box", (-120, -92, 0), (120, 92, 4), "walk"),
            ("box", (-108, -76, 4), (108, 76, 44), "wall"),
            ("box", (-108.4, -76.4, 4), (108.4, 76.4, 16), "brick"),
            ("box", (-108.4, -76.4, 32), (108.4, 76.4, 36), "red"),
            ("box", (-110, -78, 44), (110, 78, 48), "cap"),
            ("box", (-104, -72, 45), (104, 72, 48.4), "roof"),
            ("box", (-92, -76.4, 8), (16, -73, 28), "glass"),
            ("box", (28, -76.4, 8), (64, -73, 28), "glass"),
            ("box", (-108.4, -56, 8), (-105, 24, 28), "glass"),
            ("box", (24, -84, 24), (68, -76, 30), "awning"),
            ("box", (-84, -77.4, 37), (4, -75, 43), "red"),
            ("box", (-40, 0, 48), (-8, 24, 64), "unit"),
            ("box", (-80, -40, 48), (-60, -24, 56), "unit"),
            ("box", (20, -48, 48), (44, -32, 56), "unit"),
            ("box", (40, 20, 48), (68, 36, 60), "unit"),
            ("cylz", (72, -40, 48), 4, 8, "unit"),
            ("cylz", (-20, -48, 48), 2, 8, "unit"),
        ],
    },
}

PEOPLE = {
    # height in game units (PLAYER_HEIGHT_3D = 30), sprite target height in px
    "pedestrian_boy_hero": {"height": 30.0, "px": 48},
    "pedestrian_commuter": {"height": 34.0, "px": 48},
}


# ---------- helpers ----------

def srgb_to_linear(hexstr):
    c = [int(hexstr[i:i + 2], 16) / 255 for i in (0, 2, 4)]
    return [x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c] + [1.0]


def flat_material(name, hexstr, cull=False):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = srgb_to_linear(hexstr)
    bsdf.inputs["Roughness"].default_value = 0.9
    mat.diffuse_color = srgb_to_linear(hexstr)
    mat.use_backface_culling = cull
    return mat


def image_material(name, img, alpha_clip):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nt = mat.node_tree
    bsdf = nt.nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = 1.0
    tex = nt.nodes.new("ShaderNodeTexImage")
    tex.image = img
    tex.interpolation = "Closest"  # glTF exporter writes a NEAREST sampler
    nt.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    if alpha_clip:
        rnd = nt.nodes.new("ShaderNodeMath")
        rnd.operation = "ROUND"  # exporter maps this to alphaMode MASK
        nt.links.new(tex.outputs["Alpha"], rnd.inputs[0])
        nt.links.new(rnd.outputs[0], bsdf.inputs["Alpha"])
    return mat


def new_object(name, bm, materials):
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    for m in materials:
        me.materials.append(m)
    ob = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(ob)
    return ob


def add_outline(bm, thickness, mat_index):
    """Inverted hull: copy surface, push out along box-normalised vertex normals, flip."""
    bm.normal_update()
    geom = bmesh.ops.duplicate(bm, geom=list(bm.verts) + list(bm.edges) + list(bm.faces))
    new_verts = [e for e in geom["geom"] if isinstance(e, bmesh.types.BMVert)]
    new_faces = [e for e in geom["geom"] if isinstance(e, bmesh.types.BMFace)]
    for v in new_verts:
        n = v.normal
        m = max(abs(n.x), abs(n.y), abs(n.z)) or 1.0
        v.co += n / m * thickness  # box corners move t along each axis, not t along diagonal
    for f in new_faces:
        f.material_index = mat_index
    bmesh.ops.reverse_faces(bm, faces=new_faces)


# ---------- lowpoly ----------

def build_lowpoly(spec, mats, names):
    bm = bmesh.new()
    for part in spec["parts"]:
        kind, colour = part[0], part[-1]
        if kind == "box":
            lo, hi = Vector(part[1]), Vector(part[2])
            mtx = Matrix.Translation((lo + hi) / 2) @ Matrix.Diagonal((*(hi - lo), 1))
            verts = bmesh.ops.create_cube(bm, size=1.0, matrix=mtx)["verts"]
        elif kind == "wheel":
            c, r, w = part[1], part[2], part[3]
            mtx = Matrix.Translation(c) @ Matrix.Rotation(math.pi / 2, 4, "X")
            verts = bmesh.ops.create_cone(bm, cap_ends=True, segments=10, radius1=r,
                                          radius2=r, depth=w, matrix=mtx)["verts"]
        else:  # cylz
            c, r, h = part[1], part[2], part[3]
            mtx = Matrix.Translation((c[0], c[1], c[2] + h / 2))
            verts = bmesh.ops.create_cone(bm, cap_ends=True, segments=8, radius1=r,
                                          radius2=r, depth=h, matrix=mtx)["verts"]
        for f in {f for v in verts for f in v.link_faces}:
            f.material_index = names.index(colour)
    add_outline(bm, spec["outline"], len(names))
    return new_object("lowpoly", bm, mats)


# ---------- voxel ----------

def voxelize(spec, names):
    v = spec["voxel"]
    cells = {}
    for part in spec["parts"]:
        kind, colour = part[0], names.index(part[-1])
        if kind == "box":
            lo, hi = part[1], part[2]
        elif kind == "wheel":
            c, r, w = part[1], part[2], part[3]
            lo, hi = (c[0] - r, c[1] - w / 2, c[2] - r), (c[0] + r, c[1] + w / 2, c[2] + r)
        else:
            c, r, h = part[1], part[2], part[3]
            lo, hi = (c[0] - r, c[1] - r, c[2]), (c[0] + r, c[1] + r, c[2] + h)
        rng = [range(math.floor(lo[a] / v), math.ceil(hi[a] / v)) for a in range(3)]
        for i in rng[0]:
            x = (i + 0.5) * v
            if not lo[0] <= x < hi[0]:
                continue
            for j in rng[1]:
                y = (j + 0.5) * v
                if not lo[1] <= y < hi[1]:
                    continue
                for k in rng[2]:
                    z = (k + 0.5) * v
                    if not lo[2] <= z < hi[2]:
                        continue
                    if kind == "wheel" and (x - c[0]) ** 2 + (z - c[2]) ** 2 > r * r:
                        continue
                    if kind == "cylz" and (x - c[0]) ** 2 + (y - c[1]) ** 2 > r * r:
                        continue
                    cells[(i, j, k)] = colour
    return cells


DIRS = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)]


def build_voxel(spec, mats, names):
    v = spec["voxel"]
    cells = voxelize(spec, names)
    bm = bmesh.new()
    vert_cache = {}

    def vert(p):
        if p not in vert_cache:
            vert_cache[p] = bm.verts.new((p[0] * v, p[1] * v, p[2] * v))
        return vert_cache[p]

    for (i, j, k), colour in cells.items():
        for d in DIRS:
            if (i + d[0], j + d[1], k + d[2]) in cells:
                continue  # hidden face
            axis = next(a for a in range(3) if d[a])
            u, w = [a for a in range(3) if a != axis]
            base = [i, j, k]
            if d[axis] > 0:
                base[axis] += 1
            quad = []
            for du, dw in ((0, 0), (1, 0), (1, 1), (0, 1)):
                p = list(base)
                p[u] += du
                p[w] += dw
                quad.append(vert(tuple(p)))
            f = bm.faces.new(quad)
            f.material_index = colour
            f.normal_update()
            if f.normal.dot(Vector(d)) < 0:
                f.normal_flip()
    add_outline(bm, spec["outline"], len(names))
    return new_object("voxel", bm, mats), len(cells)


# ---------- sprites ----------

def load_sprite(name, target_px):
    """Key out magenta, crop, nearest-downscale to target_px tall. Returns RGBA (h,w,4), top row first."""
    img = bpy.data.images.load(str(SRC / "people" / f"{name}.png"))
    w, h = img.size
    px = np.empty(w * h * 4, dtype=np.float32)
    img.pixels.foreach_get(px)
    px = px.reshape(h, w, 4)[::-1]  # Blender stores bottom-up
    bpy.data.images.remove(img)
    r, g, b = px[..., 0], px[..., 1], px[..., 2]
    fg = ~((r - g > 0.2) & (b - g > 0.2))  # magenta + its antialiased fringe
    ys, xs = np.nonzero(fg)
    y0, y1, x0, x1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
    scale = (y1 - y0) / target_px
    out_w = max(1, round((x1 - x0) / scale))
    sy = (y0 + (np.arange(target_px) + 0.5) * scale).astype(int)
    sx = (x0 + (np.arange(out_w) + 0.5) * scale).astype(int)
    rgba = px[np.ix_(sy, sx)].copy()
    rgba[..., 3] = fg[np.ix_(sy, sx)]
    rgba[rgba[..., 3] == 0, :3] = 0
    return rgba


def sprite_image(name, rgba):
    h, w = rgba.shape[:2]
    img = bpy.data.images.new(name, w, h, alpha=True)
    img.pixels.foreach_set(rgba[::-1].ravel())
    img.pack()
    return img


def build_billboard(name, info):
    rgba = load_sprite(name, info["px"])
    h, w = rgba.shape[:2]
    s = info["height"] / h
    img = sprite_image(f"{name}_sprite", rgba)
    bm = bmesh.new()
    uv = bm.loops.layers.uv.new()
    hw = w * s / 2
    # Vertical quad in XZ, facing -Y (Blender) = +Z (glTF/Bevy), origin at the feet.
    vs = [bm.verts.new(p) for p in ((-hw, 0, 0), (hw, 0, 0), (hw, 0, h * s), (-hw, 0, h * s))]
    f = bm.faces.new(vs)
    if f.normal.y > 0:
        f.normal_flip()
    for loop in f.loops:
        co = loop.vert.co
        loop[uv].uv = ((co.x + hw) / (2 * hw), co.z / (h * s))
    return new_object("billboard", bm, [image_material("sprite", img, alpha_clip=True)])


def build_extruded(name, info, depth=4.0):
    rgba = load_sprite(name, info["px"])
    h, w = rgba.shape[:2]
    s = info["height"] / h
    img = sprite_image(f"{name}_sprite", rgba)
    solid = rgba[..., 3] > 0
    bm = bmesh.new()
    uv = bm.loops.layers.uv.new()
    for row in range(h):
        for col in range(w):
            if not solid[row, col]:
                continue
            x0, x1 = (col - w / 2) * s, (col + 1 - w / 2) * s
            z0, z1 = (h - row - 1) * s, (h - row) * s
            uc, vc = (col + 0.5) / w, (h - row - 0.5) / h
            quads = [  # (corners, outward normal, textured?) front/back faces show the texel
                ([(x0, -depth / 2, z0), (x1, -depth / 2, z0), (x1, -depth / 2, z1), (x0, -depth / 2, z1)],
                 (0, -1, 0), True),
                ([(x0, depth / 2, z0), (x1, depth / 2, z0), (x1, depth / 2, z1), (x0, depth / 2, z1)],
                 (0, 1, 0), True),
            ]
            for dc, dr, xs, zs, n in ((-1, 0, (x0, x0), (z0, z1), (-1, 0, 0)),
                                      (1, 0, (x1, x1), (z0, z1), (1, 0, 0)),
                                      (0, -1, (x0, x1), (z1, z1), (0, 0, 1)),
                                      (0, 1, (x0, x1), (z0, z0), (0, 0, -1))):
                rr, cc = row + dr, col + dc
                if 0 <= rr < h and 0 <= cc < w and solid[rr, cc]:
                    continue
                a, b = (xs[0], zs[0]), (xs[1], zs[1])
                quads.append(([(a[0], -depth / 2, a[1]), (b[0], -depth / 2, b[1]),
                               (b[0], depth / 2, b[1]), (a[0], depth / 2, a[1])], n, False))
            for corners, n, textured in quads:
                f = bm.faces.new([bm.verts.new(c) for c in corners])
                f.normal_update()
                if f.normal.dot(Vector(n)) < 0:
                    f.normal_flip()
                for loop in f.loops:
                    co = loop.vert.co
                    loop[uv].uv = ((co.x / s + w / 2) / w, co.z / s / h) if textured else (uc, vc)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-4)
    return new_object("extruded", bm, [image_material("sprite", img, alpha_clip=False)])


# ---------- scene / preview ----------

def setup_scene(ob):
    scene = bpy.context.scene
    for eng in ("BLENDER_EEVEE", "BLENDER_EEVEE_NEXT"):
        try:
            scene.render.engine = eng
            break
        except TypeError:
            continue
    scene.render.filter_size = 0.0  # no AA: hard pixel edges
    scene.render.film_transparent = True
    scene.view_settings.view_transform = "Standard"
    scene.unit_settings.scale_length = 0.1  # 10 game units = 1 m (docs/isometric-view.md)

    world = bpy.data.worlds.new("World")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs["Color"].default_value = (0.55, 0.55, 0.6, 1)
    world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.8
    scene.world = world

    sun = bpy.data.objects.new("Sun", bpy.data.lights.new("Sun", "SUN"))
    sun.data.energy = 3.0
    sun.rotation_euler = (math.radians(50), 0, math.radians(-20))  # from the camera side, slightly left
    scene.collection.objects.link(sun)

    cam = bpy.data.objects.new("PixelCam", bpy.data.cameras.new("PixelCam"))
    cam.data.type = "ORTHO"
    cam.rotation_euler = (math.radians(90 - PITCH_DEG), 0, math.radians(-YAW_DEG))
    scene.collection.objects.link(cam)
    scene.camera = cam

    # Fit camera to the object's bounding box in view space.
    bpy.context.view_layer.update()
    rot = cam.rotation_euler.to_matrix()
    inv = rot.transposed()
    corners = [ob.matrix_world @ Vector(c) for c in ob.bound_box]
    vc = [inv @ c for c in corners]
    xs, ys = [p.x for p in vc], [p.y for p in vc]
    w, h = max(xs) - min(xs), max(ys) - min(ys)
    centre = sum(corners, Vector()) / 8
    cam.location = centre - (rot @ Vector((0, 0, -1))) * 2000
    cam.data.clip_end = 5000
    pad = 1.1
    cam.data.ortho_scale = max(w, h) * pad
    return scene, cam, w * pad, h * pad


def render_preview(scene, cam, w, h, upp, path, upscale_to=512):
    """Render at `upp` game units per pixel, then nearest-upscale for viewing."""
    side = max(w, h)
    res = max(8, math.ceil(side / upp))
    scene.render.resolution_x = scene.render.resolution_y = res
    scene.render.resolution_percentage = 100
    scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)
    img = bpy.data.images.load(str(path))
    px = np.empty(res * res * 4, dtype=np.float32)
    img.pixels.foreach_get(px)
    k = max(1, upscale_to // res)
    big = px.reshape(res, res, 4).repeat(k, 0).repeat(k, 1)
    out = bpy.data.images.new(path.stem + "_big", res * k, res * k, alpha=True)
    out.pixels.foreach_set(big.ravel())
    out.filepath_raw = str(path)
    out.file_format = "PNG"
    out.save()
    bpy.data.images.remove(img)
    bpy.data.images.remove(out)


def finish(name, category, variant, ob, upp):
    scene, cam, w, h = setup_scene(ob)
    PREVIEWS.mkdir(parents=True, exist_ok=True)
    render_preview(scene, cam, w, h, upp, PREVIEWS / f"{name}_{variant}.png")
    render_preview(scene, cam, w, h, INGAME_UNITS_PER_PX, PREVIEWS / f"{name}_{variant}_ingame.png")
    out = OUT / category / f"{name}_{variant}.blend"
    out.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(out), compress=True)
    print(f"saved {out.relative_to(ROOT)}")


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.preferences.filepaths.save_version = 0  # no .blend1 backups on re-runs


def main():
    for name, spec in ASSETS.items():
        names = list(spec["palette"])
        for variant in ("lowpoly", "voxel"):
            reset()
            mats = [flat_material(n, spec["palette"][n]) for n in names]
            mats.append(flat_material("outline", OUTLINE_HEX, cull=True))
            if variant == "lowpoly":
                ob = build_lowpoly(spec, mats, names)
            else:
                ob, n = build_voxel(spec, mats, names)
                print(f"{name}: {n} voxels")
            ob.name = name
            finish(name, spec["category"], variant, ob, spec["detail_upp"])

    for name, info in PEOPLE.items():
        for variant, build in (("billboard", build_billboard), ("extruded", build_extruded)):
            reset()
            ob = build(name, info)
            ob.name = name
            if variant == "billboard":
                # Preview the billboard as Bevy would draw it: turned to face the camera.
                ob.rotation_euler.z = math.radians(-YAW_DEG)
            finish(name, "people", variant, ob, 0.4)


main()
