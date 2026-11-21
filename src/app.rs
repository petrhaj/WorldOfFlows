use crate::flows::calculate_and_route_flows;
use crate::network::generate_network;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    generate_network()?;
    calculate_and_route_flows(
        "data/spatial/world_lod0.gpkg",
        "data/spatial/world_lod0_routed.gpkg",
    )?;
    Ok(())
}
