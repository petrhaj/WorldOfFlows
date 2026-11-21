mod app;
mod flows;
mod network;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
