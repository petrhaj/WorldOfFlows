import sqlite3
import struct

DATABASE = "data/spatial/world_lod0.gpkg"


def gpkg_header():
    # GeoPackage Binary header: GP magic, version, flags, EPSG:4326 SRID
    return b"GP" + bytes([0x00, 0x01]) + struct.pack("<i", 4326)


def point(lon, lat):
    wkb = bytes([0x01]) + struct.pack("<I", 1) + struct.pack("<dd", lon, lat)
    return gpkg_header() + wkb


def line(ax, ay, bx, by):
    wkb = bytes([0x01]) + struct.pack("<I", 2) + struct.pack("<I", 2)
    wkb += struct.pack("<dd", ax, ay) + struct.pack("<dd", bx, by)
    return gpkg_header() + wkb


con = sqlite3.connect(DATABASE)
node_coordinates = {}

for node_id, lon, lat in con.execute(
    "SELECT node_id, longitude, latitude FROM nodes"
):
    node_coordinates[node_id] = (lon, lat)
    con.execute("UPDATE nodes SET geom=? WHERE node_id=?", (point(lon, lat), node_id))

for link_id, from_node, to_node in con.execute(
    "SELECT link_id, from_node, to_node FROM links"
):
    ax, ay = node_coordinates[from_node]
    bx, by = node_coordinates[to_node]
    con.execute("UPDATE links SET geom=? WHERE link_id=?", (line(ax, ay, bx, by), link_id))

con.commit()
print("node geometry bytes:", con.execute("SELECT length(geom) FROM nodes LIMIT 1").fetchone()[0])
print("link geometry bytes:", con.execute("SELECT length(geom) FROM links LIMIT 1").fetchone()[0])
con.close()
