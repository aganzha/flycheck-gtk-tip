use async_channel::Sender;
use cairo::{Context, Format, ImageSurface};
use emacs::{defun, Env, Result, Value};
use glib::ffi as glib_ffi;
use glib::translate::*;
use gtk::ffi;
use gtk::glib;
use gtk::prelude::*;
use pango::FontDescription;
//(module-load (expand-file-name "/home/aganzha/emacs-gtk3-module/target/release/libemacs_gtk3_module.so"))
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Once, OnceLock, RwLock};

emacs::plugin_is_GPL_compatible!();

static INIT: Once = Once::new();

static SENDER: OnceLock<RwLock<Sender<Event>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct Tip {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub font: String,
    pub font_size: i32,
}

pub enum Event {
    HideTip,
    ShowTip(Tip),
}

fn render_text_offscreen(
    text: &str,
    font: &str,
    size: f64,
    max_width: i32,
) -> (ImageSurface, f64, f64) {
    let tmp = ImageSurface::create(Format::ARgb32, 1, 1).unwrap();
    let cr = Context::new(&tmp).unwrap();
    let layout = pangocairo::functions::create_layout(&cr);
    layout.set_text(text);
    let desc = FontDescription::from_string(&format!("{} {}", font, size));
    layout.set_font_description(Some(&desc));
    layout.set_width(pango::SCALE * max_width); // <-- set width here too

    let (w, h) = layout.pixel_size(); // <-- use pixel_size, not pixel_extents

    let surface = ImageSurface::create(Format::ARgb32, w, h).unwrap();
    let cr = Context::new(&surface).unwrap();
    let layout = pangocairo::functions::create_layout(&cr);
    layout.set_text(text);
    layout.set_font_description(Some(&desc));
    layout.set_width(pango::SCALE * max_width); // <-- and here

    // what it was???
    // same foreground color as in main view
    //cr.set_source_rgba(1.0, 1.0, 1.0, 0.0);
    //cr.paint().unwrap();
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.move_to(0.0, 0.0);
    pangocairo::functions::show_layout(&cr, &layout);

    (surface, w as f64, h as f64)
}

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
                                                 //let k = 1.0 + t * (padding / radius.max(1.0)); // scale-like inflation

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
        cr.set_source_rgba(0.0, 0.0, 0.0, alpha);
        build_popover_path(cr, w2, h2, arrow_x2, r2, a2);
        cr.fill();
        cr.restore();
    }
}

fn draw_popover(cr: &cairo::Context, w: f64, h: f64, arrow_x: f64, radius: f64, arrow_size: f64) {
    //println!("🧄 draw_popover");
    build_popover_path(cr, w, h, arrow_x, radius, arrow_size);

    // fill with same background as main window!
    cr.set_source_rgb(0.17, 0.21, 0.26);
    cr.fill_preserve();

    // final thin outline
    cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
    cr.set_line_width(1.0);
    cr.stroke();
}

#[emacs::module(name = "emacs-gtk3-module")]
fn init<'a>(env: &'a Env) -> Result<Value<'a>> {
    INIT.call_once(|| {
        let _ = gtk::init();
    });
    let (sender, receiver) = async_channel::unbounded();
    SENDER.get_or_init(|| RwLock::new(sender.clone()));

    let text_surface = ImageSurface::create(Format::ARgb32, 1, 1).unwrap();
    let canvas = Rc::new(RefCell::new((text_surface, 1.0, 1.0)));
    let padding = 20.0;
    let radius = 12.0;
    let arrow_size = 14.0;
    let arrow_x = 60.0;

    let emacs_window = get_emacs_window();
    eprintln!("♦️................ {:?}", emacs_window);
    let window = gtk::Window::builder()
        .type_(gtk::WindowType::Popup)
        .type_hint(gtk::gdk::WindowTypeHint::Tooltip)
        .window_position(gtk::WindowPosition::Mouse)
        .build();
    window.set_decorated(false);
    //window.set_default_size(content_w as i32, total_h as i32);
    window.set_resizable(true);
    window.set_app_paintable(true);

    window.set_transient_for(emacs_window.clone().as_ref());
    eprintln!("‼️................ {:?}", window);
    window.move_(0, 0);

    let area = gtk::DrawingArea::new();
    //area.set_size_request(content_w as i32, total_h as i32);

    area.connect_draw({
        let canvas = canvas.clone();
        let window = window.clone();
        move |_, cr| {
            // Clear to transparent
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
            cr.set_operator(cairo::Operator::Source);
            cr.paint().unwrap();
            cr.set_operator(cairo::Operator::Over); // restore default

            let (ref text_surface, tw, th) = *canvas.borrow();
            let content_w = tw + padding * 2.0;
            let content_h = th + padding * 2.0;
            eprintln!("♦️ >>>>>>>>>>>>>>>> tw {} th {}", content_w, content_h);

            // choose shadow parameters
            let shadow_pad = 24.0;
            let shadow_steps = 10;
            let dx = 0.0; // like css shadow offset-x
            let dy = 10.0; // like css shadow offset-y

            cr.translate(shadow_pad, shadow_pad);

            // shadow
            draw_shadow(
                &cr,
                content_w,
                content_h,
                arrow_x,
                radius,
                arrow_size,
                shadow_pad,
                shadow_steps,
                dx,
                dy,
            );

            window.resize((content_w + shadow_pad*3.0) as i32, (content_h + shadow_pad*3.0) as i32);
            // was draw_popover_shape
            draw_popover(cr, content_w, content_h, arrow_x, radius, arrow_size);

            // Fill shape
            cr.set_source_rgb(0.95, 0.95, 0.95);
            cr.fill_preserve().unwrap();

            // Stroke shape outline
            cr.set_source_rgb(0.7, 0.7, 0.7);
            cr.set_line_width(1.0);
            cr.stroke().unwrap();

            // Draw the text on top
            cr.set_source_surface(&*text_surface, padding, padding + arrow_size)
                .unwrap();
            cr.paint().unwrap();

            gtk::glib::signal::Propagation::Stop
        }
    });

    window.add(&area);

    glib::spawn_future_local(async move {
        while let Ok(event) = receiver.recv().await {
            match event {
                Event::HideTip => {
                    window.hide();
                }
                Event::ShowTip(tip) => {
                    let max_width = emacs_window
                        .clone()
                        .map(|w| w.size().0 - tip.x)
                        .unwrap_or(300);
                    eprintln!("🧣 show tip. max_width {:?}", max_width);
                    let (text_surface, tw, th) = render_text_offscreen(
                        &tip.text,
                        &tip.font,
                        tip.font_size as f64,
                        max_width,
                    );
                    canvas.replace((text_surface, tw, th));
                    window.show_all();
                    area.queue_draw();
                    window.set_opacity(0.5);
                    window.queue_draw();
                    window.move_(
                        (tip.x as f64 - arrow_x) as i32,
                        (tip.y as f64 + radius + arrow_size * 2.0 + padding) as i32,
                    );
                    glib::timeout_add_local(std::time::Duration::from_millis(16), {
                        let target = window.clone();
                        move || {
                            let opacity = target.opacity();
                            if opacity < 1.0 {
                                target.set_opacity((opacity + 0.05).min(1.0));
                                return glib::ControlFlow::Continue;
                            }
                            glib::ControlFlow::Break
                        }
                    });
                }
            }
        }
    });
    env.intern("t")
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

// (emacs-gtk3-module-move-window 300 300 "привет!")
#[defun]
fn show_tip(
    env: &Env,
    x: i32,
    y: i32,
    text: String,
    font: String,
    font_size: i32,
) -> Result<Value<'_>> {
    eprintln!("💨 font_size {:?}", font_size);
    if let Some(lock) = SENDER.get() {
        let sender = lock.read().unwrap();
        sender
            .send_blocking(Event::ShowTip(Tip {
                x,
                y,
                text,
                font,
                font_size: font_size / 2,
            }))
            .expect("cant send through channel");
    }
    env.intern("t")
}
#[defun]
fn hide_tip(env: &Env) -> Result<Value<'_>> {
    if let Some(lock) = SENDER.get() {
        let sender = lock.read().unwrap();
        sender
            .send_blocking(Event::HideTip)
            .expect("cant send through channel");
    }
    env.intern("t")
}
