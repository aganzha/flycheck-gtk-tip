use async_channel::Sender;
use cairo::{Context, Format, ImageSurface};
use emacs::{defun, Env, Result, Value};
use glib::translate::*;
use gtk::ffi;
use gtk::glib;
use glib::ffi as glib_ffi;
use gtk::prelude::*;
use pango::FontDescription;
use pangocairo;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Once, OnceLock, RwLock};

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
fn show_window<'a>(env: &'a Env, frame: Value<'a>) -> Result<Value<'a>> {
    eprintln!(">>>>>>>>>>>>>>>>>>>>> env {:?} value {:?}", env, frame);
    //let parent = unsafe { gtk::Window::from_glib_none(frame) };
    INIT.call_once(|| {
        let _ = gtk::init();
    });
    let (sender, receiver) = async_channel::unbounded();
    SENDER.get_or_init(|| RwLock::new(sender.clone()));

    //let win = gtk::Window::new(gtk::WindowType::Toplevel);
    //win.set_type_hint(gtk::gdk::WindowTypeHint::Popup);
    let (text_surface, tw, th) = render_text_offscreen("Hello from thread", "Sans", 24.0);
    let canvas = Rc::new(RefCell::new(text_surface));
    let padding = 20.0;
    let radius = 12.0;
    let arrow_size = 14.0;
    let arrow_x = 60.0;

    let content_w = tw as f64 + padding * 2.0;
    let content_h = th as f64 + padding * 2.0;
    let total_h = content_h + arrow_size;

    let emacs_window = get_emacs_window();
    eprintln!("♦️................ {:?}", emacs_window);
    let window = gtk::Window::builder()
        .type_(gtk::WindowType::Popup)
        .type_hint(gtk::gdk::WindowTypeHint::Tooltip)
        .window_position(gtk::WindowPosition::Mouse)
        .build(); //new(gtk::WindowType::Popup);
    //window.set_type_hint(gtk::gdk::WindowTypeHint::Tooltip);
    window.set_decorated(false);
    window.set_default_size(content_w as i32, total_h as i32);
    window.set_resizable(false);
    window.set_app_paintable(true);

    window.set_transient_for(emacs_window.as_ref());
    eprintln!("‼️................ {:?}", window);
    window.move_(600, 10);
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

    area.connect_draw({
        let canvas = canvas.clone();
        move |_, cr| {
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

            let text_surface = canvas.borrow();
            // Draw the text on top
            cr.set_source_surface(&*text_surface, padding, padding + arrow_size)
                .unwrap();
            cr.paint().unwrap();

            gtk::glib::signal::Propagation::Stop
        }
    });

    event_box.add(&area);
    window.add(&event_box);
    window.show_all();
    eprintln!("screen = {:?}", WidgetExt::screen(&window));

    glib::spawn_future_local(async move {
        while let Ok(event) = receiver.recv().await {
            match event {
                Event::Test => {
                    eprintln!("🐦 xtest vent! {:?}", window);
                }
                Event::Best(title) => {
                    //window.set_title(&title);
                    eprintln!(
                        "🧣 BEST event. rust created window = {:?} and title {:?}",
                        window, title
                    );
                    let (text_surface, _tw, _th) = render_text_offscreen("thats me!", "Sans", 24.0);
                    canvas.replace(text_surface);
                    area.queue_draw();
                    window.queue_draw();
                    window.move_(110, 10);
                    eprintln!("mooooooooooooooooooooo");
                }
            }
        }
    });
    Ok(env.intern("t")?)
}

fn get_emacs_window() -> Option<gtk::Window> {
    let list = unsafe { ffi::gtk_window_list_toplevels() };
    println!("List pointer: {:p}", list);
    if !list.is_null() {
        let first = unsafe { (*list).data };
        println!("🧶 First window pointer: {:?}", first);
        let win = unsafe { gtk::Window::from_glib_none(first as *mut ffi::GtkWindow) };
        println!("🧄 win {:?} title {:?}", win, win.title());
        // Don't free the list here if you still need the window -
        // from_glib_none increments the refcount, so the window stays alive.
        // aganzha commented out
        unsafe { glib_ffi::g_list_free(list) };
        return Some(win);
    }
    None
}
// ;;(module-load (expand-file-name "/home/aganzha/emacs-gtk3-module/target/release/libemacs_gtk3_module.so"))
// ;;(emacs-gtk3-module-show-window)
// ;;(emacs-gtk3-module-set-window-title "hey")

#[defun]
fn set_window_title(env: &Env, title: String) -> Result<Value<'_>> {
    eprintln!("💨 set_window_title! {:?}", &title);
    // let list = unsafe { ffi::gtk_window_list_toplevels() };
    // println!("List pointer: {:p}", list);
    // if !list.is_null() {
    //     let first = unsafe { (*list).data };
    //     println!("🧶 First window pointer: {:?}", first);
    //     let win = unsafe { gtk::Window::from_glib_none(first as *mut ffi::GtkWindow) };
    //     println!("🧄 win {:?} title {:?}", win, win.title());
    //     // Don't free the list here if you still need the window -
    //     // from_glib_none increments the refcount, so the window stays alive.
    //     // aganzha commented out
    //     unsafe { glib_ffi::g_list_free(list) };
    // }
    if let Some(lock) = SENDER.get() {
        let sender = lock.read().unwrap();
        sender
            .send_blocking(Event::Best(title))
            .expect("cant send through channel");
    }
    Ok(env.intern("t")?)
}
