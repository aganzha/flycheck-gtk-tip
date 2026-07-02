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

pub struct Popover {
    width: f64,
    height: f64,
    radius: f64,
    arrow_size: f64,
    arrow_x: f64,
}

impl Popover {
    fn draw_path(&self, cr: &cairo::Context) {
        let arrow_half = self.arrow_size / 2.0;
        cr.new_path();
        cr.move_to(self.radius, self.arrow_size);
        cr.line_to(self.arrow_x - arrow_half, self.arrow_size);
        cr.line_to(self.arrow_x, 0.0);
        cr.line_to(self.arrow_x + arrow_half, self.arrow_size);
        cr.line_to(self.width - self.radius, self.arrow_size);

        cr.arc(
            self.width - self.radius,
            self.arrow_size + self.radius,
            self.radius,
            -std::f64::consts::FRAC_PI_2,
            0.0,
        );

        cr.line_to(self.width, self.height - self.radius);
        cr.arc(
            self.width - self.radius,
            self.height - self.radius,
            self.radius,
            0.0,
            std::f64::consts::FRAC_PI_2,
        );

        cr.line_to(self.radius, self.height);
        cr.arc(
            self.radius,
            self.height - self.radius,
            self.radius,
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::PI,
        );

        cr.line_to(0.0, self.arrow_size + self.radius);

        cr.arc(
            self.radius,
            self.arrow_size + self.radius,
            self.radius,
            std::f64::consts::PI,
            std::f64::consts::PI * 1.5,
        );
        cr.close_path();
    }
}

pub struct TextCanvas {
    surface: ImageSurface,
    fg_color: String,
    bg_color: String,
    width: f64,
    height: f64,

    padding: f64,
    radius: f64,
    arrow_size: f64,
    arrow_x: f64,

    shadow_pad: f64,
    shadow_steps: i32,
    dx: f64,
    dy: f64,
}

impl Default for TextCanvas {
    fn default() -> Self {
        Self {
            surface: ImageSurface::create(Format::ARgb32, 1, 1).unwrap(),
            fg_color: "black".to_string(),
            bg_color: "white".to_string(),
            width: 1.0,
            height: 1.0,

            padding: 20.0,
            radius: 12.0,
            arrow_size: 14.0,
            arrow_x: 60.0,

            shadow_pad: 24.0,
            shadow_steps: 10,
            dx: 5.0,
            dy: 10.0,
        }
    }
}

impl TextCanvas {
    fn prepare_text(&mut self, tip: &Tip, max_width: i32) {
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
        self.surface = surface;
        self.width = w as f64;
        self.height = h as f64;
        self.bg_color = tip.bg_color.clone();
        self.bg_color = tip.bg_color.clone();
    }

    fn window_position(&self, tip: &Tip, has_titlebar: bool) -> (i32, i32) {
        let window_x: i32 = {
            let target_x = (tip.x as f64 - self.arrow_x) as i32;
            if target_x > 0 {
                target_x
            } else {
                0
            }
        };
        let mut window_y = (tip.y as f64 + self.arrow_size + self.padding) as i32;
        if has_titlebar {
            window_y += TITLE_BAR_HEIGHT;
        }
        (window_x, window_y)
    }
    fn window_size(&self) -> (i32, i32) {
        (
            (self.full_width() + self.shadow_pad) as i32,
            (self.full_height() + self.shadow_pad + self.arrow_size) as i32,
        )
    }
    fn popover(&self) -> Popover {
        Popover {
            width: self.full_width(),
            height: self.full_height(),
            radius: self.radius,
            arrow_size: self.arrow_size,
            arrow_x: self.arrow_x,
        }
    }
    fn full_width(&self) -> f64 {
        self.width + self.padding * 2.0
    }
    fn full_height(&self) -> f64 {
        self.height + self.padding * 2.0
    }
    fn draw_popover(&self, cr: &cairo::Context) {
        self.popover().draw_path(cr);

        let bg_rgba = gdk::RGBA::parse(&self.bg_color).unwrap();

        cr.set_source_rgb(bg_rgba.red(), bg_rgba.green(), bg_rgba.blue());
        cr.fill_preserve();

        //final thin outline
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
        cr.set_line_width(1.0);
        cr.stroke();
    }
    fn draw_shadow(&self, cr: &cairo::Context) {
        for i in 0..self.shadow_steps {
            let t = i as f64 / (self.shadow_steps as f64 - 1.0);

            let pad = t * self.padding;

            let w2 = self.full_width() + 2.0 * pad;
            let h2 = self.full_height() + 2.0 * pad;
            let r2 = (self.radius + pad).max(0.0);
            let a2 = (self.arrow_size + pad).max(0.0);
            let arrow_x2 = self.arrow_x + pad;

            let alpha = (1.0 - t).powi(2) * 0.20;
            cr.save();
            cr.translate(self.dx - pad, self.dy - pad);
            cr.set_source_rgba(0.2, 0.0, 0.0, alpha); // <- shadow color here
            let popover = Popover {
                width: w2,
                height: h2,
                radius: r2,
                arrow_x: arrow_x2,
                arrow_size: a2,
            };
            popover.draw_path(cr);
            cr.fill();
            cr.restore();
        }
    }
}

const TITLE_BAR_HEIGHT: i32 = 35;

fn has_titlebar(window: &gtk::Window) -> bool {
    if let Some(gdk_win) = window.window() {
        let state = gdk_win.state();
        !state.contains(gdk::WindowState::FULLSCREEN)
    } else {
        true
    }
}

#[emacs::module(name = "flycheck-gtk-tip")]
fn init<'a>(env: &'a Env) -> Result<Value<'a>> {
    INIT.call_once(|| {
        let _ = gtk::init();
    });
    let (sender, receiver) = async_channel::unbounded();
    SENDER.get_or_init(|| RwLock::new(sender.clone()));

    let canvas = Rc::new(RefCell::new(TextCanvas::default()));

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
            canvas.draw_shadow(cr);

            let (window_w, window_h) = canvas.window_size();
            window.resize(window_w, window_h);

            canvas.draw_popover(cr);

            cr.set_source_surface(
                &*canvas.surface,
                canvas.padding,
                canvas.padding + canvas.arrow_size,
            )
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

                    {
                        canvas.borrow_mut().prepare_text(&tip, max_width);
                    }

                    window.show_all();
                    area.queue_draw();

                    let (window_x, window_y) = canvas.borrow().window_position(&tip, has_titlebar);
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
    if !list.is_null() {
        let first = unsafe { (*list).data };
        let win = unsafe { gtk::Window::from_glib_none(first as *mut ffi::GtkWindow) };
        unsafe { glib_ffi::g_list_free(list) };
        return Some(win);
    }
    None
}

#[defun]
fn show(
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
fn hide(env: &Env) -> Result<Value<'_>> {
    if let Some(lock) = SENDER.get() {
        let sender = lock.read().unwrap();
        sender
            .send_blocking(Event::HideTip)
            .expect("cant send through channel");
    }
    env.intern("t")
}
