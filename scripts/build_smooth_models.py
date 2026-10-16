"""Build "normal" smooth-shaded 3D models of the pixel-art assets, meant to be made
pixel-art by a render effect (low-res render + nearest upscale + outline + palette).

Run: blender -b -P scripts/build_smooth_models.py

Writes art/models/<category>/<asset>_smooth.blend and art/models/previews/<asset>_smooth.png
(panels: normal render | pixel effect | pixel + palette quantise | in-game scale).

Structure for animation in Bevy:
  cars      root empty -> body + wheel_FL/FR/RL/RR child objects, origin at each hub,
            so Bevy can find them by Name and spin/steer them procedurally.
  buildings root empty -> building + fan_N child objects (spin about Z).
  people    armature (hips/spine/head/arm.L/R/leg.L/R) with one skinned mesh and
            Walk + Idle actions, exported as glTF animations (AnimationPlayer).
"""

import math
import sys
from pathlib import Path

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from build_pixel_models import (  # noqa: E402
    INGAME_UNITS_PER_PX, OUT, PREVIEWS, reset, setup_scene, srgb_to_linear,
)

# Parts: ("box", lo, hi, col) | ("prism", [(x, z)...], y0, y1, col)  (side profile extruded in Y)
#        | ("cyl", centre, radius, length, axis, col) | ("sphere", centre, (rx, ry, rz), col)

TIRE, RIM = "tire", "rim"


def wheel(c, r, w):
    return [("cyl", c, r, w, "Y", TIRE), ("cyl", c, r * 0.6, w + 0.3, "Y", RIM),
            ("cyl", c, r * 0.2, w + 0.7, "Y", "tire")]


def car(body_parts, wheels, r, w):
    """wheels: (x_front, x_rear, y, z). Returns groups {name: (pivot, parts)}."""
    xf, xr, y, z = wheels
    groups = {"body": ((0, 0, 0), body_parts)}
    for name, c in (("wheel_FL", (xf, y, z)), ("wheel_FR", (xf, -y, z)),
                    ("wheel_RL", (xr, y, z)), ("wheel_RR", (xr, -y, z))):
        groups[name] = (c, wheel(c, r, w))
    return groups


def mirror_y(parts):
    """Part list plus its mirror across y=0 (boxes/cyls/spheres only)."""
    out = list(parts)
    for p in parts:
        if p[0] == "box":
            (x0, y0, z0), (x1, y1, z1) = p[1], p[2]
            out.append(("box", (x0, -y1, z0), (x1, -y0, z1), p[3]))
        elif p[0] == "prism":
            out.append(("prism", p[1], -p[3], -p[2], p[4]))
        elif p[0] in ("cyl", "sphere"):
            c = p[1]
            out.append((p[0], (c[0], -c[1], c[2]), *p[2:]))
    return out


CAR_PAL = dict(tire=("222428", 0, 0.8), rim=("b8bcc0", 1.0, 0.3), glass=("1e2a36", 0, 0.05),
               trim=("2a2c30", 0, 0.6), tail=("d02a20", 0, 0.3), head=("f2e8c0", 0, 0.2),
               plate=("dfe6f0", 0, 0.5), chrome=("c8ccd0", 1.0, 0.25))

ASSETS = {
    "car_silver_sedan": dict(
        category="cars", bevel=0.8, detail_upp=0.5,
        palette=dict(CAR_PAL, body=("b5bcc6", 0.6, 0.35)),
        groups=car(
            [("prism", [(-24, 4), (24, 4), (24.5, 8.5), (23, 10.5), (11, 11.8), (4.5, 18.2),
                        (-8, 18.2), (-14, 12.6), (-23, 12.2), (-24.5, 9)], -12, 12, "body"),
             ("prism", [(11.4, 11.8), (5.1, 18.0), (-8.6, 18.0), (-14.4, 12.6)], -10.5, 10.5, "glass"),
             *mirror_y([
                 ("prism", [(9.5, 12.6), (4.4, 17.4), (-7.6, 17.4), (-12.6, 12.9)], 11.6, 12.25, "glass"),
                 ("box", (-25, 7, 8.5), (-23.5, 11, 10.5), "tail"),
                 ("box", (23.6, 7, 8), (24.9, 11, 9.6), "head"),
                 ("box", (8, 12, 12), (10, 14, 13.6), "body"),
             ]),
             ("box", (-25, -3, 6), (-24.2, 3, 8), "plate"),
             ("box", (-25.2, -12, 3.5), (-23.5, 12, 5.5), "trim"),
             ("box", (23.5, -12, 3.5), (25.2, 12, 5.5), "trim")],
            (15, -15, 11.5, 5), 5, 4),
    ),
    "car_red_pickup": dict(
        category="cars", bevel=0.8, detail_upp=0.5,
        palette=dict(CAR_PAL, body=("c0201c", 0.5, 0.35), bed=("4a0e0c", 0, 0.8)),
        groups=car(
            [("prism", [(-28, 5), (28, 5), (28.5, 10), (27, 14), (13, 15), (-4, 15), (-4, 10),
                        (-28, 10)], -14, 14, "body"),
             ("box", (-28, -14, 10), (-4, -12, 15), "body"),
             ("box", (-28, 12, 10), (-4, 14, 15), "body"),
             ("box", (-28, -12, 10), (-26, 12, 15), "body"),
             ("box", (-26, -12, 9.6), (-4, 12, 10.4), "bed"),
             ("prism", [(-4, 15), (13, 15), (9.5, 23.5), (-3.5, 23.5)], -13, 13, "body"),
             ("prism", [(13.4, 15), (10.0, 23.2), (8, 23.2), (8, 15)], -11.5, 11.5, "glass"),
             ("box", (-4.4, -10, 16.5), (-3.6, 10, 22), "glass"),
             *mirror_y([
                 ("prism", [(11.8, 16), (9.2, 22.5), (-2.8, 22.5), (-2.8, 16)], 12.6, 13.25, "glass"),
                 ("box", (3.5, 12.5, 16), (4.5, 13.4, 22.5), "body"),
                 ("box", (-28.8, 10, 10.5), (-27.6, 13.5, 14.5), "tail"),
                 ("box", (27.6, 9, 10), (28.8, 13, 12.5), "head"),
                 ("box", (13, 13, 16), (15, 15.5, 18.5), "chrome"),
             ]),
             ("box", (-30, -14, 3), (-28, 14, 7), "chrome"),
             ("box", (28, -14, 3), (30, 14, 7), "chrome"),
             ("box", (-30.2, -3, 4), (-29.6, 3, 6.5), "plate")],
            (17, -17, 12.5, 6), 6, 5),
    ),
    "car_box_truck": dict(
        category="cars", bevel=0.8, detail_upp=0.6,
        palette=dict(CAR_PAL, cab=("f2b020", 0.3, 0.4), box=("e8ecf0", 0, 0.6),
                     door=("9aa0a8", 0.4, 0.5), chassis=("303438", 0, 0.7)),
        groups=car(
            [("box", (-38, -12, 4), (38, 12, 8), "chassis"),
             ("prism", [(22, 6), (38.5, 6), (38.5, 17), (37, 26), (22, 27)], -14, 14, "cab"),
             ("prism", [(38.9, 17.5), (37.5, 25.5), (36, 25.5), (36, 17.5)], -12, 12, "glass"),
             *mirror_y([
                 ("box", (28, 13.6, 17), (35, 14.3, 25), "glass"),
                 ("box", (38, 14, 18), (39, 16, 23), "chassis"),
                 ("box", (-38.6, 14, 10), (-37.8, 15.6, 15), "tail"),
                 ("box", (38.4, 10, 9), (39, 13, 12), "head"),
             ]),
             ("box", (-38, -16, 8), (20, 16, 38), "box"),
             ("box", (-38.5, -14, 10), (-37.8, 14, 36), "door"),
             *[("box", (-38.9, -14, z), (-38.3, 14, z + 0.6)) + ("chassis",) for z in range(12, 36, 3)],
             ("box", (-40, -14, 2), (-38, 14, 4), "chassis")],
            (30, -22, 12, 6), 6, 6),
    ),
}

FF_PAL = dict(wall=("d9c6a5", 0, 0.9), stone=("8c8478", 0, 0.95), purple=("5a3a8c", 0, 0.6),
              red=("c0282a", 0, 0.6), roof=("8e8e96", 0, 0.95), glass=("5a86b0", 0.2, 0.1),
              unit=("a8b0b8", 0.6, 0.4), pole=("2a2a35", 0.5, 0.5), cream=("f2ebd0", 0, 0.5),
              grille=("3a3e44", 0.5, 0.5))
PH_PAL = dict(walk=("b0aaa0", 0, 0.95), wall=("d2a878", 0, 0.9), brick=("9a4a32", 0, 0.95),
              red=("d02828", 0, 0.5), cap=("e2c69a", 0, 0.8), roof=("9a9aa0", 0, 0.95),
              glass=("7aa0c0", 0.2, 0.1), awning=("2e6a48", 0, 0.7), unit=("a0a8b0", 0.6, 0.4),
              grille=("3a3e44", 0.5, 0.5))


def ring(x0, y0, x1, y1, z0, z1, t, col):
    """Hollow parapet: four walls of thickness t, so the roof inside stays visible."""
    return [("box", (x0, y0, z0), (x1, y0 + t, z1), col), ("box", (x0, y1 - t, z0), (x1, y1, z1), col),
            ("box", (x0, y0, z0), (x0 + t, y1, z1), col), ("box", (x1 - t, y0, z0), (x1, y1, z1), col)]


def fan(name, c, r):
    return {name: (c, [("cyl", c, r, 0.8, "Z", "grille"),
                       ("box", (c[0] - r, c[1] - 0.6, c[2]), (c[0] + r, c[1] + 0.6, c[2] + 1), "unit"),
                       ("box", (c[0] - 0.6, c[1] - r, c[2]), (c[0] + 0.6, c[1] + r, c[2] + 1), "unit")])}


ASSETS["building_fast_food_joint"] = dict(
    category="buildings", bevel=0.5, detail_upp=1.2, palette=FF_PAL,
    groups={"building": ((0, 0, 0), [
        ("box", (-68, -52, 0), (68, 52, 32), "wall"),
        ("box", (-68.4, -52.4, 0), (68.4, 52.4, 8), "stone"),
        ("box", (-68.4, -52.4, 20), (68.4, 52.4, 24), "red"),
        ("box", (-68.4, -52.4, 24), (68.4, 52.4, 28), "purple"),
        *ring(-69, -53, 69, 53, 32, 36, 4, "purple"),
        ("box", (-65, -49, 32), (65, 49, 33), "roof"),
        ("box", (0, -52.6, 8), (56, -51.5, 20), "glass"),
        *[("box", (x - 0.6, -53, 8), (x + 0.6, -51.6, 20), "wall") for x in (14, 28, 42)],
        ("box", (-68.6, -40, 8), (-67.5, 40, 20), "glass"),
        *[("box", (-69, y - 0.6, 8), (-67.6, y + 0.6, 20), "wall") for y in (-20, 0, 20)],
        ("box", (-40, -56, 0), (-12, -48, 44), "wall"),
        ("box", (-40.4, -56.4, 0), (-11.6, -47.6, 8), "stone"),
        ("box", (-41, -57, 32), (-11, -47, 40), "purple"),
        ("box", (-34, -56.6, 8), (-18, -55.5, 24), "glass"),
        ("box", (-26.6, -57, 8), (-25.4, -55.6, 24), "pole"),
        ("box", (36, -56, 0), (60, -48, 40), "red"),
        ("box", (43, -56.6, 14), (53, -55.5, 24), "glass"),
        ("box", (41, -61, 24), (55, -56, 25.5), "purple"),
        ("box", (16, 6, 36), (36, 26, 44), "unit"),
        ("box", (-30, -14, 36), (-10, 6, 44), "unit"),
        ("cyl", (-48, 24, 40), 3.5, 8, "Z", "unit"),
        ("cyl", (-48, 24, 44.5), 5, 1.5, "Z", "unit"),
        ("cyl", (-32, 32, 40), 3.5, 8, "Z", "unit"),
        ("cyl", (-32, 32, 44.5), 5, 1.5, "Z", "unit"),
        ("cyl", (-96, -70, 32), 2.5, 64, "Z", "pole"),
        ("box", (-112, -72, 62), (-80, -68, 92), "purple"),
        ("cyl", (-96, -70, 92), 16, 4, "Y", "purple"),
        ("box", (-109, -72.6, 65), (-83, -67.4, 92), "cream"),
        ("cyl", (-96, -70, 92), 13, 5.2, "Y", "cream"),
    ]), **fan("fan_1", (26, 16, 44), 7), **fan("fan_2", (-20, -4, 44), 7)},
)

ASSETS["building_strip_mall_pharmacy"] = dict(
    category="buildings", bevel=0.5, detail_upp=1.5, palette=PH_PAL,
    groups={"building": ((0, 0, 0), [
        ("box", (-120, -92, 0), (120, 92, 3), "walk"),
        ("box", (-108, -76, 3), (108, 76, 44), "wall"),
        ("box", (-108.4, -76.4, 3), (108.4, 76.4, 16), "brick"),
        ("box", (-108.4, -76.4, 32), (108.4, 76.4, 36), "red"),
        *ring(-110, -78, 110, 78, 44, 48, 5, "cap"),
        ("box", (-105, -73, 44), (105, 73, 45), "roof"),
        ("box", (-92, -76.6, 8), (16, -75.5, 28), "glass"),
        *[("box", (x - 0.6, -77, 8), (x + 0.6, -75.6, 28), "wall") for x in range(-80, 16, 12)],
        ("box", (28, -76.6, 8), (64, -75.5, 28), "glass"),
        ("box", (45.4, -77, 8), (46.6, -75.6, 28), "wall"),
        ("box", (-108.6, -56, 8), (-107.5, 24, 28), "glass"),
        ("box", (24, -84, 26), (68, -76, 29), "awning"),
        ("box", (24, -84.4, 23), (68, -83.6, 29), "awning"),
        ("box", (-84, -77.6, 37), (4, -76, 43), "red"),
        ("box", (-40, 0, 47), (-8, 24, 64), "unit"),
        ("box", (-80, -40, 47), (-60, -24, 56), "unit"),
        ("box", (20, -48, 47), (44, -32, 56), "unit"),
        ("cyl", (72, -40, 51), 3, 8, "Z", "unit"),
        ("cyl", (-20, -48, 51), 1.5, 8, "Z", "unit"),
        ("cyl", (-60, 40, 51), 1.5, 8, "Z", "unit"),
    ]), **fan("fan_1", (-70, -32, 56), 6), **fan("fan_2", (32, -40, 56), 6),
        **fan("fan_3", (54, 28, 47), 9)},
)


def person(pal, s, extras=()):
    """Bones and skinned parts for a figure facing +X. s scales the 30-unit boy layout."""
    def k(p):
        return tuple(v * s for v in p)
    bones = {  # name: (head, tail, parent)
        "hips": (k((0, 0, 12.5)), k((0, 0, 14.5)), None),
        "spine": (k((0, 0, 14.5)), k((0, 0, 21)), "hips"),
        "head": (k((0, 0, 21)), k((0, 0, 29)), "spine"),
        "arm.L": (k((0, 5.3, 21)), k((0, 5.3, 11)), "spine"),
        "arm.R": (k((0, -5.3, 21)), k((0, -5.3, 11)), "spine"),
        "leg.L": (k((0, 2, 13)), k((0, 2, 1)), "hips"),
        "leg.R": (k((0, -2, 13)), k((0, -2, 1)), "hips"),
    }
    parts = {b: [] for b in bones}
    for side, y in (("L", 1), ("R", -1)):
        parts[f"leg.{side}"] += [
            ("cyl", k((0, 2 * y, 7.5)), 1.7 * s, 11 * s, "Z", "legs"),
            ("box", k((-1.8, 2 * y - 1.7, 0)), k((3.4, 2 * y + 1.7, 2.2)), "shoes"),
        ]
        parts[f"arm.{side}"] += [
            ("cyl", k((0, 5.3 * y, 19)), 1.5 * s, 4.4 * s, "Z", "sleeve"),
            ("cyl", k((0, 5.4 * y, 14.5)), 1.1 * s, 6 * s, "Z", "skin"),
            ("sphere", k((0, 5.4 * y, 11)), k((1.3, 1.3, 1.5)), "skin"),
        ]
    parts["hips"] += [("box", k((-2.4, -4.2, 12)), k((2.6, 4.2, 14)), "belt")]
    parts["spine"] += [("box", k((-2.2, -4.2, 13.8)), k((2.4, 4.2, 21)), "torso"),
                       ("cyl", k((0, 0, 21.3)), 1.1 * s, 1.6 * s, "Z", "skin")]
    parts["head"] += [
        ("sphere", k((0.3, 0, 25)), k((3.4, 3.2, 3.8)), "skin"),
        ("sphere", k((-0.5, 0, 26.6)), k((3.6, 3.5, 3.1)), "hair"),
        ("sphere", k((3.5, 1.2, 25.4)), k((0.45, 0.45, 0.6)), "eyes"),
        ("sphere", k((3.5, -1.2, 25.4)), k((0.45, 0.45, 0.6)), "eyes"),
    ]
    for bone, part in extras:
        parts[bone].append(part)
    return dict(palette=pal, bones=bones, groups={b: ((0, 0, 0), p) for b, p in parts.items()})


ASSETS["pedestrian_boy_hero"] = dict(
    category="people", bevel=0.25, detail_upp=0.3,
    **person(dict(skin=("d89a72", 0, 0.7), hair=("6a4028", 0, 0.8), eyes=("1a1420", 0, 0.3),
                  torso=("9a9ea4", 0, 0.9), sleeve=("9a9ea4", 0, 0.9), belt=("5a4030", 0, 0.6),
                  legs=("2e5c8a", 0, 0.9), shoes=("6a4430", 0, 0.7)), 1.0),
)
_s = 34 / 30
ASSETS["pedestrian_commuter"] = dict(
    category="people", bevel=0.25, detail_upp=0.34,
    **person(dict(skin=("d8a080", 0, 0.7), hair=("4a3020", 0, 0.8), eyes=("1a1420", 0, 0.3),
                  torso=("2c3a5c", 0, 0.8), sleeve=("2c3a5c", 0, 0.8), belt=("2c3a5c", 0, 0.8),
                  legs=("4a4440", 0, 0.9), shoes=("1e1c1e", 0, 0.4), shirt=("e8ecf0", 0, 0.8),
                  bag=("6a4430", 0, 0.7), cup=("f0ece0", 0, 0.6)), _s,
           extras=[("spine", ("box", (2.3 * _s, -1.2 * _s, 15 * _s), (2.6 * _s, 1.2 * _s, 21 * _s), "shirt")),
                   ("spine", ("box", (-1.5 * _s, -6.6 * _s, 11 * _s), (2.5 * _s, -5.2 * _s, 15 * _s), "bag")),
                   ("spine", ("box", (2.3 * _s, -4 * _s, 15 * _s), (2.7 * _s, -2.8 * _s, 21 * _s), "bag")),
                   ("arm.L", ("cyl", (1.2 * _s, 5.4 * _s, 11.5 * _s), 1.0 * _s, 2.6 * _s, "Z", "cup"))]),
)


# ---------- geometry ----------

def material(name, spec):
    hexstr, metal, rough = spec
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = srgb_to_linear(hexstr)
    bsdf.inputs["Metallic"].default_value = metal
    bsdf.inputs["Roughness"].default_value = rough
    mat.diffuse_color = srgb_to_linear(hexstr)
    return mat


AXES = {"X": Matrix.Rotation(math.pi / 2, 4, "Y"), "Y": Matrix.Rotation(math.pi / 2, 4, "X"),
        "Z": Matrix.Identity(4)}


def add_part(bm, part):
    kind = part[0]
    if kind == "box":
        lo, hi = Vector(part[1]), Vector(part[2])
        mtx = Matrix.Translation((lo + hi) / 2) @ Matrix.Diagonal((*(hi - lo), 1))
        return bmesh.ops.create_cube(bm, size=1.0, matrix=mtx)["verts"]
    if kind == "cyl":
        c, r, length, axis = part[1:5]
        mtx = Matrix.Translation(c) @ AXES[axis]
        return bmesh.ops.create_cone(bm, cap_ends=True, segments=20, radius1=r, radius2=r,
                                     depth=length, matrix=mtx)["verts"]
    if kind == "sphere":
        mtx = Matrix.Translation(part[1]) @ Matrix.Diagonal((*part[2], 1))
        return bmesh.ops.create_uvsphere(bm, u_segments=16, v_segments=10, radius=1.0,
                                         matrix=mtx)["verts"]
    pts, y0, y1 = part[1:4]  # prism
    a = [bm.verts.new((x, y0, z)) for x, z in pts]
    b = [bm.verts.new((x, y1, z)) for x, z in pts]
    faces = [bm.faces.new(a), bm.faces.new(b[::-1])]
    faces += [bm.faces.new((a[i], a[i - 1], b[i - 1], b[i])) for i in range(len(pts))]
    bmesh.ops.recalc_face_normals(bm, faces=faces)
    return a + b


def build_group(name, pivot, parts, names, mats, vgroups=None):
    bm = bmesh.new()
    for part in parts:
        verts = add_part(bm, part)
        for f in {f for v in verts for f in v.link_faces}:
            f.material_index = names.index(part[-1])
    bmesh.ops.translate(bm, verts=bm.verts, vec=-Vector(pivot))
    for f in bm.faces:
        f.smooth = True
    for e in bm.edges:  # smooth by angle, baked into the mesh so it survives export
        if len(e.link_faces) == 2 and e.calc_face_angle(0) > math.radians(35):
            e.smooth = False
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    for m in mats:
        me.materials.append(m)
    ob = bpy.data.objects.new(name, me)
    ob.location = pivot
    bpy.context.scene.collection.objects.link(ob)
    return ob


def add_bevel(ob, width):
    mod = ob.modifiers.new("Bevel", "BEVEL")
    mod.width = width
    mod.segments = 2
    mod.limit_method = "ANGLE"
    mod.angle_limit = math.radians(35)


def build_rigid(name, spec, names, mats):
    """Root empty with one child object per group (body, wheels, fans)."""
    root = bpy.data.objects.new(name, None)
    bpy.context.scene.collection.objects.link(root)
    obs = []
    for gname, (pivot, parts) in spec["groups"].items():
        ob = build_group(gname, pivot, parts, names, mats)
        ob.parent = root
        add_bevel(ob, spec["bevel"])
        obs.append(ob)
    return obs


def build_rigged(name, spec, names, mats):
    """Armature + one skinned mesh, each part weighted 100% to its bone."""
    arm = bpy.data.armatures.new(name + "_rig")
    rig = bpy.data.objects.new(name, arm)
    bpy.context.scene.collection.objects.link(rig)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    for bname, (head, tail, parent) in spec["bones"].items():
        eb = arm.edit_bones.new(bname)
        eb.head, eb.tail = head, tail
        if parent:
            eb.parent = arm.edit_bones[parent]
    bpy.ops.object.mode_set(mode="OBJECT")

    bm = bmesh.new()
    deform = bm.verts.layers.deform.verify()
    bone_names = list(spec["bones"])
    for bname, (_, parts) in spec["groups"].items():
        for part in parts:
            verts = add_part(bm, part)
            for v in verts:
                v[deform][bone_names.index(bname)] = 1.0
            for f in {f for v in verts for f in v.link_faces}:
                f.material_index = names.index(part[-1])
    for f in bm.faces:
        f.smooth = True
    for e in bm.edges:
        if len(e.link_faces) == 2 and e.calc_face_angle(0) > math.radians(35):
            e.smooth = False
    me = bpy.data.meshes.new(name + "_mesh")
    bm.to_mesh(me)
    bm.free()
    for m in mats:
        me.materials.append(m)
    body = bpy.data.objects.new(name + "_mesh", me)
    for bname in bone_names:  # order matches the deform-layer indices above
        body.vertex_groups.new(name=bname)
    bpy.context.scene.collection.objects.link(body)
    body.parent = rig
    body.modifiers.new("Armature", "ARMATURE").object = rig
    add_bevel(body, spec["bevel"])
    body.modifiers.move(1, 0)  # bevel before armature
    add_actions(rig)
    return [body]


def add_actions(rig):
    """Walk (16-frame loop) and Idle (32-frame loop). Limb bones point down, so their
    local Z axis is the ground-plane Y axis: rotating about it swings them fore/aft."""
    pb = rig.pose.bones
    for b in pb:
        b.rotation_mode = "XYZ"
    rig.animation_data_create()

    def key(frame, poses):
        for bname, (rot_z, lift) in poses.items():
            pb[bname].rotation_euler = (0, 0, math.radians(rot_z))
            pb[bname].location = (0, lift, 0)  # hips: local Y = world up
            pb[bname].keyframe_insert("rotation_euler", frame=frame)
            pb[bname].keyframe_insert("location", frame=frame)

    def stride(a):
        return {"leg.L": (a, 0), "leg.R": (-a, 0), "arm.L": (-0.8 * a, 0), "arm.R": (0.8 * a, 0)}

    walk = [(1, stride(25), 0), (5, stride(0), 0.6), (9, stride(-25), 0), (13, stride(0), 0.6),
            (17, stride(25), 0)]
    for frame, poses, bob in walk:
        key(frame, {**poses, "hips": (0, bob), "head": (0, 0), "spine": (0, 0)})
    rig.animation_data.action.name = "Walk"
    rig.animation_data.action.use_fake_user = True

    rig.animation_data.action = None
    for frame, a, bob in ((1, 2, 0), (17, -2, 0.15), (33, 2, 0)):
        key(frame, {"leg.L": (0, 0), "leg.R": (0, 0), "arm.L": (a, 0), "arm.R": (-a, 0),
                    "hips": (0, bob), "spine": (0, 0), "head": (a, 0)})
    rig.animation_data.action.name = "Idle"
    rig.animation_data.action.use_fake_user = True

    rig.animation_data.action = bpy.data.actions["Walk"]
    bpy.context.scene.frame_set(1)  # full stride, so previews show the rig working


# ---------- pixel effect ----------

def use_pixel_effect(scene, on):
    scene.render.filter_size = 0.0 if on else 1.5
    scene.render.use_freestyle = on  # stands in for a Bevy edge-detect post-process
    if on:
        scene.render.line_thickness_mode = "ABSOLUTE"
        scene.render.line_thickness = 1.0
        fs = bpy.context.view_layer.freestyle_settings
        ls = fs.linesets[0] if fs.linesets else fs.linesets.new("Outline")
        ls.linestyle = ls.linestyle or bpy.data.linestyles.new("Outline")
        ls.linestyle.color = (0.04, 0.03, 0.05)


def render_np(scene, res):
    tmp = PREVIEWS / "_tmp.png"
    scene.render.resolution_x = scene.render.resolution_y = res
    scene.render.resolution_percentage = 100
    scene.render.filepath = str(tmp)
    bpy.ops.render.render(write_still=True)
    img = bpy.data.images.load(str(tmp))
    px = np.empty(res * res * 4, dtype=np.float32)
    img.pixels.foreach_get(px)
    bpy.data.images.remove(img)
    tmp.unlink()
    return px.reshape(res, res, 4)


def quantize(a, k=12):
    """k-means palette reduction: what a Bevy palette post-pass would do."""
    m = a[..., 3] > 0.5
    x = a[m][:, :3]
    if len(x) < k:
        return a
    centres = x[np.argsort(x.sum(1))][np.linspace(0, len(x) - 1, k).astype(int)]
    for _ in range(10):
        labels = ((x[:, None] - centres[None]) ** 2).sum(-1).argmin(1)
        for j in range(k):
            if (labels == j).any():
                centres[j] = x[labels == j].mean(0)
    out = a.copy()
    out[m, :3] = centres[labels]
    out[m, 3] = 1.0
    return out


def panel(a, size=384):
    k = max(1, size // a.shape[0])
    a = a.repeat(k, 0).repeat(k, 1)[:size, :size]
    bg = np.ones((size, size, 4), np.float32) * [0.85, 0.85, 0.85, 1]
    h, w = a.shape[:2]
    oy, ox = (size - h) // 2, (size - w) // 2
    al = a[..., 3:4]
    bg[oy:oy + h, ox:ox + w] = a * al + bg[oy:oy + h, ox:ox + w] * (1 - al)
    return bg


def render_strip(scene, w, h, upp, path):
    side = max(w, h)
    use_pixel_effect(scene, False)
    normal = render_np(scene, 384)
    use_pixel_effect(scene, True)
    pixel = render_np(scene, max(8, math.ceil(side / upp)))
    scene.render.use_freestyle = False  # at ~12px tall, lines swamp the figure
    ingame = render_np(scene, max(8, math.ceil(side / INGAME_UNITS_PER_PX)))
    scene.render.use_freestyle = True
    strip = np.concatenate([panel(normal), panel(pixel), panel(quantize(pixel)), panel(ingame)], 1)
    out = bpy.data.images.new(path.stem, strip.shape[1], strip.shape[0], alpha=True)
    out.pixels.foreach_set(strip.astype(np.float32).ravel())
    out.filepath_raw = str(path)
    out.file_format = "PNG"
    out.save()
    bpy.data.images.remove(out)
    # Leave the saved file on the pixel look: F12 shows the effect.
    scene.render.resolution_x = scene.render.resolution_y = max(8, math.ceil(side / upp))


def main():
    PREVIEWS.mkdir(parents=True, exist_ok=True)
    for name, spec in ASSETS.items():
        reset()
        names = list(spec["palette"])
        mats = [material(n, spec["palette"][n]) for n in names]
        build = build_rigged if "bones" in spec else build_rigid
        obs = build(name, spec, names, mats)
        root = obs[0].parent
        if spec["category"] == "people":
            root.rotation_euler.z = math.pi  # preview only: face the camera
        scene, cam, w, h = setup_scene(obs)
        render_strip(scene, w, h, spec["detail_upp"], PREVIEWS / f"{name}_smooth.png")
        root.rotation_euler.z = 0
        out = OUT / spec["category"] / f"{name}_smooth.blend"
        bpy.ops.wm.save_as_mainfile(filepath=str(out), compress=True)
        print(f"saved {out.relative_to(OUT.parent.parent)}")


if __name__ == "__main__":
    main()
