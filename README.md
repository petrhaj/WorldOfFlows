# World LOD0 multimodal network

This Rust project creates a deliberately simplified, connected four-mode transport network for seven continental centroids, then applies a continental passenger-tour model.

## Outputs

- `data/spatial/world_lod0.gpkg`: original network only; never modified by the flow model.
- `data/spatial/world_lod0_routed.gpkg`: copy containing `pax_flow` on air links and an `intercontinental_flows` layer.
- `data/spatial/nodes.geojson` and `data/spatial/links.geojson`: optional GeoJSON exports.

Temporary JSON files used during processing are also created under `data/spatial/` and removed after use.

## Modules

- `src/app.rs`: coordinates application startup and the two main processing stages.
- `src/network/mod.rs`: creates nodes, links, costs, times, and the original GeoPackage.
- `src/flows/mod.rs`: reads the GeoPackage, builds time/cost impedance matrices, applies the population and logit model, and creates the routed copy.

See [docs/MODEL.md](docs/MODEL.md) for the mathematical model, assumptions, and general processing flow.

## Model

The open-source population input is stored in `data/populations.csv` and uses United Nations World Population Prospects 2024 continental totals, with a simplified Antarctic research population estimate.

Each modeled person generates two tours. The model deliberately assigns 99.99% of tours to the same continent. The remaining 0.01% are distributed among cross-continent destinations with a multinomial logit using:

- size coefficient: 1.0 times `ln(destination population)`
- time coefficient: -0.01 per hour
- cost coefficient: -0.00002 per USD/tonne

Cross-continent tours are assigned to direct airport-to-airport air links and written to `data/spatial/world_lod0_routed.gpkg` as `pax_flow` values.

## Build and run

```cmd
cd /d C:\Users\petrh\Documents\WorldOfFlows
cargo run --release
python validate_routed_gpkg.py
```
