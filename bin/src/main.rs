use cairo::{Context, Format, ImageSurface};
use gtk::glib;
use gtk::prelude::*;
use pango::FontDescription;
use pangocairo;

fn build_popover_path(
    cr: &cairo::Context,
    w: f64,
    h: f64,
    arrow_x: f64,
    radius: f64,
    arrow_size: f64,
) {
    let arrow_half = arrow_size / 2.0;

    cr.new_path();
    cr.move_to(radius, arrow_size);
    cr.line_to(arrow_x - arrow_half, arrow_size);
    cr.line_to(arrow_x, 0.0);
    cr.line_to(arrow_x + arrow_half, arrow_size);
    cr.line_to(w - radius, arrow_size);

    cr.arc(
        w - radius,
        arrow_size + radius,
        radius,
        -std::f64::consts::FRAC_PI_2,
        0.0,
    );

    cr.line_to(w, h - radius);
    cr.arc(
        w - radius,
        h - radius,
        radius,
        0.0,
        std::f64::consts::FRAC_PI_2,
    );

    cr.line_to(radius, h);
    cr.arc(
        radius,
        h - radius,
        radius,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );

    cr.line_to(0.0, arrow_size + radius);

    cr.arc(
        radius,
        arrow_size + radius,
        radius,
        std::f64::consts::PI,
        std::f64::consts::PI * 1.5,
    );

    cr.close_path();
}

fn draw_shadow(
    cr: &cairo::Context,
    w: f64,
    h: f64,
    arrow_x: f64,
    radius: f64,
    arrow_size: f64,
    padding: f64, // overall shadow spread
    steps: usize, // blur smoothness
    dx: f64,
    dy: f64, // shadow offset (like box-shadow)
) {
    // shadow color: black with varying alpha
    // (you can change this to match your design)
    for i in 0..steps {
        let t = i as f64 / (steps as f64 - 1.0); // 0..1
        let k = 1.0 + t * (padding / radius.max(1.0)); // scale-like inflation

        // simpler “inflation” that often looks good:
        let pad = t * padding;

        let w2 = w + 2.0 * pad;
        let h2 = h + 2.0 * pad;
        let r2 = (radius + pad).max(0.0);
        let a2 = (arrow_size + pad).max(0.0);
        let arrow_x2 = arrow_x + pad;

        let alpha = (1.0 - t).powi(2) * 0.35; // tweak to taste

        cr.save();
        cr.translate(dx - pad, dy - pad); // keep it visually aligned while inflating
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        build_popover_path(cr, w2, h2, arrow_x2, r2, a2);
        cr.fill();
        cr.restore();
    }
}

fn draw_popover(cr: &cairo::Context, w: f64, h: f64, arrow_x: f64, radius: f64, arrow_size: f64) {
    println!("🧄 gooooooooooooooooooooooooooooo");
    build_popover_path(cr, w, h, arrow_x, radius, arrow_size);

    // example fill
    cr.set_source_rgb(0.0, 0.0, 0.0);
    cr.fill_preserve();

    // example outline (optional)
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.05);
    cr.set_line_width(1.0);
    cr.stroke();
}

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

    cr.set_source_rgba(1.0, 1.0, 1.0, 0.0);
    cr.paint().unwrap();
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.move_to(-ink.x() as f64, -ink.y() as f64);
    pangocairo::functions::show_layout(&cr, &layout);

    (surface, w, h)
}

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
    window.set_default_size((content_w) as i32, (total_h) as i32);
    window.set_resizable(false);
    window.set_app_paintable(true);

    let area = gtk::DrawingArea::new();

    // choose shadow parameters
    let shadow_pad = 24.0;
    let shadow_steps = 10;
    let dx = 0.0; // like css shadow offset-x
    let dy = 10.0; // like css shadow offset-y


    area.connect_draw({
        let area = area.clone();
        move |_, cr| {
            let w = area.allocated_width() as i32;
            let h = area.allocated_height() as i32;

            // offscreen: ARGB so alpha shadow works
            let surf = cairo::ImageSurface::create(cairo::Format::ARgb32, w, h).unwrap();
            let s_cr = cairo::Context::new(&surf).unwrap();

            // clear offscreen to transparent
            s_cr.set_operator(cairo::Operator::Source);
            s_cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
            s_cr.paint().unwrap();
            s_cr.set_operator(cairo::Operator::Over);

            // translate so the shadow isn’t clipped by widget bounds
            s_cr.translate(shadow_pad, shadow_pad);

            // shadow
            draw_shadow(
                &s_cr,
                content_w,
                total_h,
                arrow_x,
                radius,
                arrow_size,
                shadow_pad,
                shadow_steps,
                dx,
                dy,
            );

            // popover
            draw_popover(&s_cr, content_w, total_h, arrow_x, radius, arrow_size);

            s_cr.set_source_rgb(0.95, 0.95, 0.95);
            s_cr.fill_preserve().unwrap();

            s_cr.set_source_rgb(0.7, 0.7, 0.7);
            s_cr.set_line_width(1.0);
            s_cr.stroke().unwrap();

            s_cr.set_source_surface(&text_surface, padding, padding + arrow_size)
                .unwrap();
            s_cr.paint().unwrap();

            // finally paint to real cairo context
            cr.set_operator(cairo::Operator::Source);
            cr.set_source_surface(&surf, 0.0, 0.0).unwrap();
            cr.paint().unwrap();

            gtk::glib::signal::Propagation::Stop
        }
    });


    window.add(&area);
    window.show_all();
    let target = window.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
        let opacity = target.opacity();
        if opacity < 1.0 {
            target.set_opacity((opacity + 0.05).min(1.0));
            return glib::ControlFlow::Continue;
        }
        glib::ControlFlow::Break
    });
    gtk::main();
}
