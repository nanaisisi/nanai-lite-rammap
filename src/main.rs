mod category;
mod gpu;
mod memory;
mod treemap;
mod ui;

use ui::app::RammapApp;
use ui::types::AppInput;
use windows_reactor::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::run_component::<RammapApp>(AppInput)?;
    Ok(())
}
