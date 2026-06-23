use cairo::{Context, Format, ImageSurface};
use gtk::gdk;
use gtk::prelude::*;
use pango::FontDescription;
use pangocairo;

fn render_text_offscreen(text: &str, font: &str, size: f64) -> (ImageSurface, i32, i32) {
    let tmp = ImageSurface::create(Format::ARgb32, 1, 1).unwrap();
    let cr = Context::new(&tmp).unwrap();
    let layout = pangocairo::functions::create_layout(&cr);
    layout.set_text(text);
    let desc = FontDescription::from_string(&format!("{} {}", font, size));
    layout.set_font_description(Some(&desc));
    let (ink, _logical) = layout.pixel_extents();
    let w = ink.width();
    let h = ink.height();

    let surface = ImageSurface::create(Format::ARgb32, w, h).unwrap();
    let cr = Context::new(&surface).unwrap();
    let layout = pangocairo::functions::create_layout(&cr);
    layout.set_text(text);
    layout.set_font_description(Some(&desc));

    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    cr.paint().unwrap();
    cr.set_source_rgb(0.0, 0.0, 0.0);
    cr.move_to(-ink.x() as f64, -ink.y() as f64);
    pangocairo::functions::show_layout(&cr, &layout);

    (surface, w, h)
}

fn draw_popover_shape(cr: &Context, w: f64, h: f64, arrow_x: f64, radius: f64, arrow_size: f64) {
    let arrow_half = arrow_size / 2.0;

    cr.new_path();
    // Start at top-left + radius, shifted down by arrow_size
    cr.move_to(radius, arrow_size);
    // Top edge to arrow start
    cr.line_to(arrow_x - arrow_half, arrow_size);
    // Arrow pointing up (triangle) - tip at y=0
    cr.line_to(arrow_x, 0.0);
    cr.line_to(arrow_x + arrow_half, arrow_size);
    // Continue top edge to right-radius
    cr.line_to(w - radius, arrow_size);
    // Top-right corner
    cr.arc(
        w - radius,
        arrow_size + radius,
        radius,
        -std::f64::consts::FRAC_PI_2,
        0.0,
    );
    // Right edge
    cr.line_to(w, h - radius);
    // Bottom-right corner
    cr.arc(
        w - radius,
        h - radius,
        radius,
        0.0,
        std::f64::consts::FRAC_PI_2,
    );
    // Bottom edge
    cr.line_to(radius, h);
    // Bottom-left corner
    cr.arc(
        radius,
        h - radius,
        radius,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    // Left edge
    cr.line_to(0.0, arrow_size + radius);
    // Top-left corner
    cr.arc(
        radius,
        arrow_size + radius,
        radius,
        std::f64::consts::PI,
        std::f64::consts::PI * 1.5,
    );
    cr.close_path();
}

// fn draw_popover_shape(cr: &Context, w: f64, h: f64, arrow_x: f64, radius: f64, arrow_size: f64) {
//     // Draw a rounded rect with a triangular arrow at the top
//     // Path: start at top-left + radius, go right, arrow, go right, round corner, down, etc.

//     let arrow_half = arrow_size / 2.0;

//     cr.new_path();
//     // Top edge: from left+radius to arrow start
//     cr.move_to(radius, 0.0);
//     cr.line_to(arrow_x - arrow_half, 0.0);
//     // Arrow pointing up (triangle)
//     cr.line_to(arrow_x, -arrow_size);
//     cr.line_to(arrow_x + arrow_half, 0.0);
//     // Continue top edge to right-radius
//     cr.line_to(w - radius, 0.0);
//     // Top-right corner
//     cr.arc(w - radius, radius, radius, -std::f64::consts::FRAC_PI_2, 0.0);
//     // Right edge
//     cr.line_to(w, h - radius);
//     // Bottom-right corner
//     cr.arc(w - radius, h - radius, radius, 0.0, std::f64::consts::FRAC_PI_2);
//     // Bottom edge
//     cr.line_to(radius, h);
//     // Bottom-left corner
//     cr.arc(radius, h - radius, radius, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
//     // Left edge
//     cr.line_to(0.0, radius);
//     // Top-left corner
//     cr.arc(radius, radius, radius, std::f64::consts::PI, std::f64::consts::PI * 1.5);
//     cr.close_path();
// }

fn main() {
    gtk::init().unwrap();

    let (text_surface, tw, th) = render_text_offscreen("Hello from thread", "Sans", 24.0);

    let padding = 20.0;
    let radius = 12.0;
    let arrow_size = 14.0;
    let arrow_x = 60.0;

    let content_w = tw as f64 + padding * 2.0;
    let content_h = th as f64 + padding * 2.0;
    let total_h = content_h + arrow_size;

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_decorated(false);
    window.set_default_size(content_w as i32, total_h as i32);
    window.set_resizable(false);
    window.set_app_paintable(true);

    // Make window transparent via CSS
    let provider = gtk::CssProvider::new();
    provider
        .load_from_data(b"window { background: transparent; }")
        .unwrap();
    let screen = WidgetExt::screen(&window).unwrap();

    gtk::StyleContext::add_provider_for_screen(
        &screen,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // EventBox to hold the drawing area, with RGBA visual
    let event_box = gtk::EventBox::new();
    event_box.set_visible_window(false); // don't draw its own background
    let visual = screen.rgba_visual().unwrap();
    event_box.set_visual(Some(&visual));

    let area = gtk::DrawingArea::new();
    area.set_size_request(content_w as i32, total_h as i32);

    area.connect_draw(move |_, cr| {
        // Clear to transparent
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.set_operator(cairo::Operator::Source);
        cr.paint().unwrap();
        cr.set_operator(cairo::Operator::Over); // restore default

        // Draw the popover shape
        draw_popover_shape(cr, content_w, total_h, arrow_x, radius, arrow_size);

        // Fill shape
        cr.set_source_rgb(0.95, 0.95, 0.95);
        cr.fill_preserve().unwrap();

        // Stroke shape outline
        cr.set_source_rgb(0.7, 0.7, 0.7);
        cr.set_line_width(1.0);
        cr.stroke().unwrap();

        // Draw the text on top
        cr.set_source_surface(&text_surface, padding, padding + arrow_size)
            .unwrap();
        cr.paint().unwrap();

        gtk::glib::signal::Propagation::Stop
    });

    // area.connect_draw(move |_, cr| {
    //     // Clear to transparent
    //     cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    //     //cr.set_source_rgb(0.95, 0.95, 0.95);

    //     cr.set_operator(cairo::Operator::Source);
    //     cr.paint().unwrap();

    //     // Draw the popover shape
    //     draw_popover_shape(cr, content_w, total_h, arrow_x, radius, arrow_size);
    //     cr.set_source_rgb(0.7, 0.7, 0.7);
    //     cr.set_line_width(1.0);
    //     cr.stroke().unwrap();

    //     eprintln!("popover {:?} {:?} {:?} {:?}", content_w, radius, arrow_size, total_h);
    //     // Fill with a background color

    //     cr.set_source_rgb(0.95, 0.95, 0.95);
    //     cr.fill_preserve().unwrap();

    //     // // Stroke border
    //     cr.set_source_rgb(0.7, 0.7, 0.7);
    //     cr.set_line_width(1.0);
    //     cr.stroke().unwrap();

    //     // // Draw the text on top
    //     // cr.set_source_surface(&text_surface, padding, padding + arrow_size).unwrap();
    //     // cr.paint().unwrap();

    //     gtk::glib::signal::Propagation::Stop
    // });

    event_box.add(&area);
    window.add(&event_box);
    window.show_all();
    gtk::main();
}

// fn main() {
//     gtk::init().unwrap();

//     let (text_surface, tw, th) = render_text_offscreen("Hello from thread", "Sans", 24.0);

//     let padding = 20.0;
//     let radius = 12.0;
//     let arrow_size = 14.0;
//     let arrow_x = 60.0; // where the arrow points

//     let content_w = tw as f64 + padding * 2.0;
//     let content_h = th as f64 + padding * 2.0;
//     let total_h = content_h + arrow_size; // arrow sticks out the top

//     let window = gtk::Window::new(gtk::WindowType::Toplevel);

//     window.set_decorated(false);
//     window.set_default_size(content_w as i32, total_h as i32);
//     window.set_resizable(false);
//     window.set_app_paintable(true); // we handle all drawing

//     let screen = WidgetExt::screen(&window).unwrap();

//     let provider = gtk::CssProvider::new();
//     provider.load_from_data(b"window { background: transparent; }").unwrap();

//     gtk::StyleContext::add_provider_for_screen(
//         &screen,
//         &provider,
//         gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
//     );

//     let area = gtk::DrawingArea::new();
//     area.set_size_request(content_w as i32, total_h as i32);

//     area.connect_draw(move |_, cr| {
//         // Clear to transparent
//         cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
//         cr.set_operator(cairo::Operator::Source);
//         cr.paint().unwrap();

//         // Draw the popover shape with a shadow-like fill
//         draw_popover_shape(cr, content_w, total_h, arrow_x, radius, arrow_size);

//         // Fill with a background color
//         cr.set_source_rgb(0.95, 0.95, 0.95);
//         cr.fill_preserve().unwrap();

//         // Optional: stroke border
//         cr.set_source_rgb(0.7, 0.7, 0.7);
//         cr.set_line_width(1.0);
//         cr.stroke().unwrap();

//         // Draw the text on top
//         cr.set_source_surface(&text_surface, padding, padding + arrow_size).unwrap();
//         cr.paint().unwrap();

//         gtk::glib::signal::Propagation::Stop
//     });

//     window.add(&area);
//     window.show_all();
//     gtk::main();
// }
