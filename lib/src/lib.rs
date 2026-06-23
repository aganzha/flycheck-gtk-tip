use async_channel::Sender;
use emacs::{defun, Env, Result, Value};
use gtk::glib;
use gtk::prelude::*;
use std::sync::{Once, OnceLock, RwLock};
use cairo::{Context, Format, ImageSurface};
use pango::FontDescription;
use pangocairo;

emacs::plugin_is_GPL_compatible!();

static INIT: Once = Once::new();

static SENDER: OnceLock<RwLock<Sender<Event>>> = OnceLock::new();

#[emacs::module(name = "emacs-gtk3-module")]
fn init(_env: &Env) -> Result<()> {
    Ok(())
}

pub enum Event {
    Test,
    Best(String),
}

// fn render_text_offscreen(text: &str, font: &str, size: f64) -> ImageSurface {
//     let surface = ImageSurface::create(Format::ARgb32, 400, 100).unwrap();
//     let cr = Context::new(&surface).unwrap();

//     let layout = pangocairo::functions::create_layout(&cr);//.unwrap();
//     layout.set_text(text);
//     let desc = FontDescription::from_string(&format!("{} {}", font, size));
//     layout.set_font_description(Some(&desc));

//     cr.set_source_rgb(1.0, 1.0, 1.0);
//     cr.paint().unwrap();
//     cr.set_source_rgb(0.0, 0.0, 0.0);
//     cr.move_to(10.0, 10.0);
//     pangocairo::functions::show_layout(&cr, &layout);

//     surface
// }
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


#[defun]
fn show_window(env: &Env) -> Result<Value<'_>> {
    INIT.call_once(|| {
        let _ = gtk::init();
    });
    let (sender, receiver) = async_channel::unbounded();
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

    event_box.add(&area);
    window.add(&event_box);
    window.show_all();




    // SENDER.get_or_init(|| RwLock::new(sender.clone()));
    // let win = gtk::Window::new(gtk::WindowType::Toplevel);
    // win.set_title("From Emacs (GTK3)");
    // win.set_default_size(400, 300);

    // // let btn = gtk::Button::with_label("Click me");
    // // btn.connect_clicked({
    // //     //let window = win.clone();
    // //     let sender = sender.clone();
    // //     move |_| {
    // //         eprintln!("Button clicked from Emacs module");
    // //         //eprintln!("window! {:?}", window);
    // //         sender
    // //             .send_blocking(Event::Test)
    // //             .expect("cant send through channel");
    // //     }
    // // });
    // //win.add(&btn);
    // let surface = render_text_offscreen("Hello from thread", "Sans", 24.0);
    // let area = gtk::DrawingArea::new();
    // area.connect_draw(move |_, cr| {
    //     //if let Ok(surface) = rx.try_recv() {
    //     // Only here does a window (via cr from DrawingArea) get involved
    //     cr.set_source_surface(&surface, 0.0, 0.0).unwrap();
    //     cr.paint().unwrap();
    //     //}
    //     gtk::glib::signal::Propagation::Stop
    //     //Inhibit(false)
    // });
    // win.add(&area);
    // win.show_all();

    glib::spawn_future_local(async move {
        while let Ok(event) = receiver.recv().await {
            match event {
                Event::Test => {
                    eprintln!("🐦 xtest vent! {:?}", window);
                }
                Event::Best(title) => {
                    window.set_title(&title);
                    eprintln!("🧣 BEST vent! {:?} and title {:?}", window, title);
                }
            }
        }
    });
    Ok(env.intern("t")?)
}

#[defun]
fn set_window_title(env: &Env, title: String) -> Result<Value<'_>> {
    eprintln!("💨 set_window_title! {:?}", &title);
    if let Some(lock) = SENDER.get() {
        let sender = lock.read().unwrap();
        sender
            .send_blocking(Event::Best(title))
            .expect("cant send through channel");
    }
    Ok(env.intern("t")?)
}

// ----------------------------------------------------
// // attemp1
// // it works!
// // ;;(module-load (expand-file-name "/home/aganzha/emacs-gtk3-module/target/release/libemacs_gtk3_module.so"))
// // ;;(emacs-gtk3-module-show-window)

// use emacs::{defun, Env, Result, Value};
// use gtk::prelude::*;
// use std::sync::Once;

// emacs::plugin_is_GPL_compatible!();

// static INIT: Once = Once::new();

// #[emacs::module(name = "emacs-gtk3-module")]
// fn init(_env: &Env) -> Result<()> {
//     Ok(())
// }

// #[defun]
// fn show_window(env: &Env) -> Result<Value<'_>> {
//     INIT.call_once(|| {
//         // Try to init, ignore failure if already initialized
//         let _ = gtk::init();
//         eprintln!("just inited gtk3. how?");
//     });

//     let win = gtk::Window::new(gtk::WindowType::Toplevel);
//     win.set_title("From Emacs (GTK3)");
//     win.set_default_size(400, 300);

//     let btn = gtk::Button::with_label("Click me");
//     btn.connect_clicked(|_| {
//         eprintln!("Button clicked from Emacs module");
//     });
//     win.add(&btn);

//     win.show_all();

//     Ok(env.intern("t")?)
// }

// attempt 0
// aganzha@fedora:~$ RUST_BACKTRACE=1 emacs

// thread '<unnamed>' panicked at /home/aganzha/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gtk-0.18.2/src/auto/window.rs:30:9:
// GTK has not been initialized. Call `gtk::init` first.
// stack backtrace:
//    0: __rustc::rust_begin_unwind
//    1: core::panicking::panic_fmt
//    2: gtk::auto::window::Window::new
//    3: emacs_gtk3_module::__emrs_E_show_window::extern_lambda
//    4: <unknown>
//    5: <unknown>
//    6: <unknown>
//    7: <unknown>
//    8: <unknown>
//    9: <unknown>
//   10: <unknown>
//   11: F656c6973702d2d6576616c2d6c6173742d73657870_elisp__eval_last_sexp_0
//   12: <unknown>
//   13: <unknown>
//   14: <unknown>
//   15: F6576616c2d6c6173742d73657870_eval_last_sexp_0
//   16: <unknown>
//   17: <unknown>
//   18: <unknown>
//   19: <unknown>
//   20: F636f6d6d616e642d65786563757465_command_execute_0
//   21: <unknown>
//   22: <unknown>
//   23: <unknown>
//   24: <unknown>
//   25: <unknown>
//   26: <unknown>
//   27: <unknown>
//   28: <unknown>
//   29: <unknown>
//   30: __libc_start_call_main
//   31: __libc_start_main_alias_1
//   32: <unknown>
// note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
// ^_^_

// use emacs::{defun, Env, Result, Value};
// use gtk::prelude::*;
// use std::sync::Once;

// emacs::plugin_is_GPL_compatible!();

// static INIT: Once = Once::new();

// #[emacs::module(name = "emacs-gtk3-module")]
// fn init(_env: &Env) -> Result<()> {
//     Ok(())
// }

// #[defun]
// fn show_window(env: &Env) -> Result<Value<'_>> {
//     INIT.call_once(|| {
//         // GTK3 already initialized by Emacs, nothing to do here
//     });

//     let win = gtk::Window::new(gtk::WindowType::Toplevel);
//     win.set_title("From Emacs (GTK3)");
//     win.set_default_size(400, 300);

//     let btn = gtk::Button::with_label("Click me");
//     btn.connect_clicked(|_| {
//         eprintln!("Button clicked from Emacs module");
//     });
//     win.add(&btn);

//     win.show_all();

//     Ok(env.intern("t")?)
// }
