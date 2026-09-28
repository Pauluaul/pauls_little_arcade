mod arcade_app;
use arcade_app::App;

fn main() -> std::io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal));
    Ok(())
}