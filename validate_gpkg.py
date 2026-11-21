import os
import sqlite3

DATABASE = "data/spatial/world_lod0.gpkg"
con = sqlite3.connect(DATABASE)
print("tables:", [r[0] for r in con.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY 1")])
print("nodes:", con.execute("SELECT count(*) FROM nodes").fetchone()[0])
print("links:", con.execute("SELECT count(*) FROM links").fetchone()[0])
link_columns = {row[1] for row in con.execute("PRAGMA table_info(links)")}
assert "cost_eur_per_tonne" in link_columns
assert "cost_eur_per_passenger" in link_columns
print("cost columns:", sorted(link_columns & {"cost_eur_per_tonne", "cost_eur_per_passenger"}))
print("modes:", con.execute("SELECT mode, count(*) FROM links GROUP BY mode ORDER BY mode").fetchall())
print("size:", os.path.getsize(DATABASE), "bytes")
con.close()
