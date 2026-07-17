use razermatter_lib::bridge;

fn main() -> Result<(), rs_matter::error::Error> {
    bridge::run_server()
}
