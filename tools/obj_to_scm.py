#!/usr/bin/env python3
"""Convert a SolidCraft OBJ export (with its .mtl) to the viewer's compact mesh format.

Usage: python3 -I obj_to_scm.py model.obj out.scm

The .scm file (little-endian):
  b"SCM1", u32 vertex_count, u32 index_count,
  f32 min[3], f32 max[3]                      (bounds, in the OBJ's units)
  vertex_count x { f32 pos[3], f32 normal[3], u8 rgba[4] }   (28 bytes each)
  index_count  x u32                          (triangles)
  optional body table: b"BODY", u32 body_count,
    body_count x { u32 first_index, u32 index_count, u16 name_len, name (UTF-8) }
Each OBJ object ("o" line) is one body; its triangles are one contiguous index range.
Colours come from each object's material (Kd, sRGB 0-1); a vertex is unique per
(position, normal, colour), so bodies keep their own colour where they touch.
"""
import os
import struct
import sys


def read_mtl(path):
    colours = {}
    name = None
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            p = line.split()
            if not p:
                continue
            if p[0] == "newmtl":
                name = line.strip()[len("newmtl"):].strip()
            elif p[0] == "Kd" and name is not None:
                colours[name] = tuple(max(0, min(255, round(float(x) * 255))) for x in p[1:4])
    return colours


def main(obj_path, out_path):
    mtl_path = None
    positions, normals = [], []
    vert_index = {}
    out_vertices = bytearray()
    indices = []
    colour = (200, 200, 200)
    colours = {}
    pack_vertex = struct.Struct("<6f4B").pack
    bodies = []  # [name, first_index]
    with open(obj_path, encoding="utf-8", errors="replace") as f:
        for line in f:
            tag = line[:2]
            if tag == "o ":
                bodies.append([line[2:].strip(), len(indices)])
            elif tag == "v ":
                positions.append(tuple(map(float, line.split()[1:4])))
            elif tag == "vn":
                normals.append(tuple(map(float, line.split()[1:4])))
            elif tag == "f ":
                corners = line.split()[1:]
                ids = []
                for c in corners:
                    parts = c.split("/")
                    vi = int(parts[0]) - 1
                    ni = int(parts[2]) - 1 if len(parts) > 2 and parts[2] else -1
                    key = (vi, ni, colour)
                    k = vert_index.get(key)
                    if k is None:
                        k = len(vert_index)
                        vert_index[key] = k
                        n = normals[ni] if ni >= 0 else (0.0, 0.0, 1.0)
                        out_vertices += pack_vertex(*positions[vi], *n, *colour, 255)
                    ids.append(k)
                for i in range(1, len(ids) - 1):
                    indices += (ids[0], ids[i], ids[i + 1])
            elif line.startswith("usemtl"):
                colour = colours.get(line.strip()[len("usemtl"):].strip(), (200, 200, 200))
            elif line.startswith("mtllib"):
                mtl_path = os.path.join(os.path.dirname(obj_path), line.strip()[len("mtllib"):].strip())
                colours = read_mtl(mtl_path)
    xs = [p[0] for p in positions]
    ys = [p[1] for p in positions]
    zs = [p[2] for p in positions]
    lo = (min(xs), min(ys), min(zs))
    hi = (max(xs), max(ys), max(zs))
    with open(out_path, "wb") as f:
        f.write(b"SCM1")
        f.write(struct.pack("<II", len(vert_index), len(indices)))
        f.write(struct.pack("<6f", *lo, *hi))
        f.write(out_vertices)
        f.write(struct.pack(f"<{len(indices)}I", *indices))
        if not bodies or bodies[0][1] != 0:
            bodies.insert(0, ["(unnamed)", 0])
        table = []
        for i, (name, first) in enumerate(bodies):
            end = bodies[i + 1][1] if i + 1 < len(bodies) else len(indices)
            if end > first:
                table.append((name, first, end - first))
        f.write(b"BODY" + struct.pack("<I", len(table)))
        for name, first, count in table:
            nb = name.encode("utf-8")[:65535]
            f.write(struct.pack("<IIH", first, count, len(nb)) + nb)
    print(f"{len(vert_index)} vertices, {len(indices) // 3} triangles, {len(colours)} materials, {len(table)} bodies, bounds {lo} .. {hi}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
