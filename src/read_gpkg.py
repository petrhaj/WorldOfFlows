import json
import sqlite3
import sys

source, destination = sys.argv[1], sys.argv[2]
con = sqlite3.connect(source)
con.row_factory = sqlite3.Row

nodes = [dict(row) for row in con.execute("SELECT * FROM nodes")]
links = [dict(row) for row in con.execute("SELECT * FROM links")]
con.close()

for row in nodes:
    row.pop("geom", None)
for row in links:
    row.pop("geom", None)

with open(destination, "w", encoding="utf-8") as f:
    json.dump({"nodes": nodes, "links": links}, f)
