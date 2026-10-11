//! Rustroke Toolkit: a visual designer for rustroke views.

fn main() -> Result<(), rustroke::RunError> {
    let options = rustroke::WindowOptions {
        title: "Rustroke Toolkit".to_owned(),
        inner_size: (1500.0, 900.0),
        persistence_id: Some("rustroke-toolkit".to_owned()),
        ..Default::default()
    };
    let app = rustroke_toolkit::Toolkit::new(rtk_document::sample_project());
    rustroke::run(options, app)
}
