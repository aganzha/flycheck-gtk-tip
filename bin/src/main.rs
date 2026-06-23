use gtk::prelude::*;
use cairo::{Context, Format, ImageSurface};
use pango::FontDescription;
use pangocairo;
use std::sync::mpsc;
use std::thread;

fn render_text_offscreen(text: &str, font: &str, size: f64) -> ImageSurface {
    let surface = ImageSurface::create(Format::ARgb32, 400, 100).unwrap();
    let cr = Context::new(&surface).unwrap();

    let layout = pangocairo::functions::create_layout(&cr);//.unwrap();
    layout.set_text(text);
    let desc = FontDescription::from_string(&format!("{} {}", font, size));
    layout.set_font_description(Some(&desc));

    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.paint().unwrap();
    cr.set_source_rgb(0.0, 0.0, 0.0);
    cr.move_to(10.0, 10.0);
    pangocairo::functions::show_layout(&cr, &layout);

    surface
}

fn main() {
    gtk::init().unwrap();

    //let (tx, rx) = mpsc::channel();

    // Spawn render thread -- no window involved
    // thread::spawn(move || {
    //     let surface = render_text_offscreen("Hello from thread", "Sans", 24.0);
    //     tx.send(surface).unwrap();
    // });
    let surface = render_text_offscreen("Hello from thread", "Sans", 24.0);
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    let area = gtk::DrawingArea::new();

    area.connect_draw(move |_, cr| {
        //if let Ok(surface) = rx.try_recv() {
            // Only here does a window (via cr from DrawingArea) get involved
            cr.set_source_surface(&surface, 0.0, 0.0).unwrap();
            cr.paint().unwrap();
        //}
        gtk::glib::signal::Propagation::Stop
        //Inhibit(false)
    });

    window.add(&area);
    window.show_all();
    gtk::main();
}
