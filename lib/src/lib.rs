use async_channel::Sender;
use cairo::{Context, Format, ImageSurface};
use emacs::{defun, Env, Result, Value};
use glib::ffi as glib_ffi;
use glib::translate::*;
use gtk::ffi;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use pango::FontDescription;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Once, OnceLock, RwLock};

emacs::plugin_is_GPL_compatible!();

static INIT: Once = Once::new();

static SENDER: OnceLock<RwLock<Sender<Event>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct Tip {
    text: String,
    x: i32,
    y: i32,
    font: String,
    font_size: f64,
    fg_color: String,
    bg_color: String,
}

pub enum Event {
    HideTip,
    ShowTip(Tip),
}

fn render_text_offscreen(tip: &Tip, max_width: i32) -> (ImageSurface, f64, f64) {
    let tmp = ImageSurface::create(Format::ARgb32, 1, 1).unwrap();
    let cr = Context::new(&tmp).unwrap();
    let layout = pangocairo::functions::create_layout(&cr);
    layout.set_text(&tip.text);
    let desc = FontDescription::from_string(&format!("{} {}", &tip.font, &tip.font_size));
    layout.set_font_description(Some(&desc));
    layout.set_width(pango::SCALE * max_width);

    let (w, h) = layout.pixel_size();

    let surface = ImageSurface::create(Format::ARgb32, w, h).unwrap();
    let cr = Context::new(&surface).unwrap();
    let layout = pangocairo::functions::create_layout(&cr);
    layout.set_text(&tip.text);
    layout.set_font_description(Some(&desc));
    layout.set_width(pango::SCALE * max_width);

    let fg_rgba = gdk::RGBA::parse(&tip.fg_color).unwrap();
    cr.set_source_rgb(fg_rgba.red(), fg_rgba.green(), fg_rgba.blue());
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
    for i in 0..steps {
        let t = i as f64 / (steps as f64 - 1.0);

        let pad = t * padding;

        let w2 = w + 2.0 * pad;
        let h2 = h + 2.0 * pad;
        let r2 = (radius + pad).max(0.0);
        let a2 = (arrow_size + pad).max(0.0);
        let arrow_x2 = arrow_x + pad;

        let alpha = (1.0 - t).powi(2) * 0.20;
        cr.save();
        cr.translate(dx - pad, dy - pad);
        cr.set_source_rgba(0.2, 0.0, 0.0, alpha); // <- shadow color here
        build_popover_path(cr, w2, h2, arrow_x2, r2, a2);
        cr.fill();
        cr.restore();
    }
}

fn draw_popover(
    cr: &cairo::Context,
    w: f64,
    h: f64,
    arrow_x: f64,
    radius: f64,
    arrow_size: f64,
    _fg_color: &str,
    bg_color: &str,
) {
    build_popover_path(cr, w, h, arrow_x, radius, arrow_size);

    let bg_rgba = gdk::RGBA::parse(bg_color).unwrap();

    cr.set_source_rgb(bg_rgba.red(), bg_rgba.green(), bg_rgba.blue());
    cr.fill_preserve();

    //final thin outline
    cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
    cr.set_line_width(1.0);
    cr.stroke();
}

pub struct TextCanvas {
    surface: ImageSurface,
    fg_color: String,
    bg_color: String,
    width: f64,
    height: f64,
}

impl Default for TextCanvas {
    fn default() -> Self {
        Self {
            surface: ImageSurface::create(Format::ARgb32, 1, 1).unwrap(),
            fg_color: String::new(),
            bg_color: String::new(),
            width: 1.0,
            height: 1.0,
        }
    }
}

const TITLE_BAR_HEIGHT: i32 = 35;

fn has_titlebar(window: &gtk::Window) -> bool {
    if let Some(gdk_win) = window.window() {
        let state = gdk_win.state();
        // Fullscreen windows typically don't have titlebar
        !state.contains(gdk::WindowState::FULLSCREEN)
    } else {
        true
    }
}

#[emacs::module(name = "emacs-gtk3-module")]
fn init<'a>(env: &'a Env) -> Result<Value<'a>> {
    INIT.call_once(|| {
        let _ = gtk::init();
    });
    let (sender, receiver) = async_channel::unbounded();
    SENDER.get_or_init(|| RwLock::new(sender.clone()));

    //let text_surface = ImageSurface::create(Format::ARgb32, 1, 1).unwrap();
    //let canvas = Rc::new(RefCell::new((text_surface, 1.0, 1.0)));
    let canvas = Rc::new(RefCell::new(TextCanvas::default()));
    let padding = 20.0;
    let radius = 12.0;
    let arrow_size = 14.0;
    let arrow_x = 60.0;

    let emacs_window = get_emacs_window();

    let window = gtk::Window::builder()
        .type_(gtk::WindowType::Popup)
        .type_hint(gtk::gdk::WindowTypeHint::Tooltip)
        .window_position(gtk::WindowPosition::Mouse)
        .build();
    window.set_decorated(false);

    window.set_resizable(true);
    window.set_app_paintable(true);

    window.set_transient_for(emacs_window.clone().as_ref());

    window.move_(0, 0);

    let area = gtk::DrawingArea::new();

    area.connect_draw({
        let canvas = canvas.clone();
        let window = window.clone();
        move |_, cr| {
            let canvas = canvas.borrow();
            let content_w = canvas.width + padding * 2.0;
            let content_h = canvas.height + padding * 2.0;

            let shadow_pad = 24.0;
            let shadow_steps = 10;
            let dx = 5.0; // like css shadow offset-x
            let dy = 10.0; // like css shadow offset-y

            // shadow
            draw_shadow(
                cr,
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

            window.resize(
                (content_w + shadow_pad) as i32,
                (content_h + shadow_pad + arrow_size) as i32,
            );

            draw_popover(
                cr,
                content_w,
                content_h,
                arrow_x,
                radius,
                arrow_size,
                &canvas.fg_color,
                &canvas.bg_color,
            );

            cr.set_source_surface(&*canvas.surface, padding, padding + arrow_size)
                .unwrap();
            cr.paint().unwrap();

            gtk::glib::signal::Propagation::Stop
        }
    });

    window.add(&area);
    let threshold = Rc::new(Cell::new(false));

    glib::spawn_future_local(async move {
        while let Ok(event) = receiver.recv().await {
            match event {
                Event::HideTip => {
                    window.hide();
                }
                Event::ShowTip(tip) => {
                    if threshold.get() {
                        continue;
                    }
                    threshold.replace(true);
                    glib::timeout_add_local(std::time::Duration::from_millis(300), {
                        let threshold = threshold.clone();
                        move || {
                            threshold.replace(false);
                            glib::ControlFlow::Break
                        }
                    });
                    let (emacs_width, _emacs_height, has_titlebar) = emacs_window
                        .clone()
                        .map(|w| {
                            let size = w.size();
                            (size.0, size.1, has_titlebar(&w))
                        })
                        .unwrap_or((640, 480, true));
                    let max_width = emacs_width - tip.x;

                    let (text_surface, tw, th) = render_text_offscreen(&tip, max_width);
                    canvas.replace(TextCanvas {
                        surface: text_surface,
                        width: tw,
                        height: th,
                        fg_color: tip.fg_color.clone(),
                        bg_color: tip.bg_color.clone(),
                    });

                    window.show_all();
                    area.queue_draw();
                    //println!("🤕 ................. {:?}", emacs_window.clone().map(|w| titlebar_height(&w)));
                    let window_x: i32 = {
                        let target_x = (tip.x as f64 - arrow_x) as i32;
                        if target_x > 0 {
                            target_x
                        } else {
                            0
                        }
                    };
                    let mut window_y = (tip.y as f64 + arrow_size + padding) as i32;
                    if has_titlebar {
                        window_y += TITLE_BAR_HEIGHT;
                    }
                    window.move_(window_x, window_y);
                    // Fade In effect
                    window.set_opacity(0.5);
                    window.queue_draw();
                    glib::timeout_add_local(std::time::Duration::from_millis(15), {
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
        let win = unsafe { gtk::Window::from_glib_none(first as *mut ffi::GtkWindow) };
        unsafe { glib_ffi::g_list_free(list) };
        return Some(win);
    }
    None
}

#[defun]
fn show_tip(
    env: &Env,
    x: i32,
    y: i32,
    text: String,
    font: String,
    font_size: f64,
    fg_color: String,
    bg_color: String,
) -> Result<Value<'_>> {
    if let Some(lock) = SENDER.get() {
        let sender = lock.read().unwrap();
        sender
            .send_blocking(Event::ShowTip(Tip {
                x,
                y,
                text,
                font,
                font_size,
                bg_color,
                fg_color,
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

#[defun]
fn flycheck_display_errors_in_rust(env: &Env, errors: Value) -> Result<()> {
    let err1: Result<Value> = errors.car();
    eprintln!("⚽ {:?}", env);
    eprintln!("‼️ >>>>>>>>>>>>>>>>>>>> {:?} ........ {:?}", errors, err1);
    //let pixel_pos = env.call("frame-edges", [])?;
    //let pos_x: Result<i32> = pixel_pos.car();
    if let Ok(buffer_name) = env.call("buffer-name", []) {
        eprintln!(
            "🧳 buffer_name >> {:?} <<",
            buffer_name.into_rust::<String>()?
        );
    }
    //let buffer_name: Result<Option<String>> = buffer_name.into_rust();

    Ok(())
}
