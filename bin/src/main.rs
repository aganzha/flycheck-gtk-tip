use gtk::prelude::*;
use cairo::{Context, Format, ImageSurface};
use pango::FontDescription;
use pangocairo;

fn render_text_offscreen(text: &str, font: &str, size: f64) -> (ImageSurface, i32, i32) {
    // First create a temporary surface to measure text
    let tmp = ImageSurface::create(Format::ARgb32, 1, 1).unwrap();
    let cr = Context::new(&tmp).unwrap();
    let layout = pangocairo::functions::create_layout(&cr);
    layout.set_text(text);
    let desc = FontDescription::from_string(&format!("{} {}", font, size));
    layout.set_font_description(Some(&desc));

    let (ink, _logical) = layout.pixel_extents();
    let w = ink.width();
    let h = ink.height();

    // Now create surface sized to the text
    let surface = ImageSurface::create(Format::ARgb32, w, h).unwrap();
    let cr = Context::new(&surface).unwrap();
    let layout = pangocairo::functions::create_layout(&cr);
    layout.set_text(text);
    layout.set_font_description(Some(&desc));

    // Transparent background
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    cr.paint().unwrap();
    cr.set_source_rgb(0.0, 0.0, 0.0);
    cr.move_to(-ink.x() as f64, -ink.y() as f64);
    pangocairo::functions::show_layout(&cr, &layout);

    (surface, w, h)
}

fn main() {
    gtk::init().unwrap();

    let (surface, w, h) = render_text_offscreen("Hello from thread", "Sans", 24.0);

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_decorated(false);          // no window decorations
    window.set_default_size(w, h);
    window.set_resizable(false);

    let area = gtk::DrawingArea::new();
    area.set_size_request(w, h);

    area.connect_draw(move |_, cr| {
        cr.set_source_surface(&surface, 0.0, 0.0).unwrap();
        cr.paint().unwrap();
        gtk::glib::signal::Propagation::Stop
    });

    window.add(&area);
    window.show_all();
    gtk::main();
}
