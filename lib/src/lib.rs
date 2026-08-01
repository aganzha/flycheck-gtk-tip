// SPDX-FileCopyrightText: 2026 Aleksey Ganzha <aganzha@yandex.ru>
//
// SPDX-License-Identifier: GPL-3.0-or-later

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
use std::sync::{OnceLock, RwLock};

emacs::plugin_is_GPL_compatible!();
use emacs::use_symbols;

use_symbols! { nil }

static GTK_INITIALIZED: OnceLock<bool> = OnceLock::new();
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
    level: String,
    has_titlebar: bool,
    geometry: Geometry,
    shadow: Shadow,
}

impl Tip {
    fn window_position(&self, has_titlebar: bool) -> (i32, i32) {
        let window_x: i32 = {
            let target_x = (self.x as f64 - self.geometry.arrow_x) as i32;
            if target_x > 0 {
                target_x
            } else {
                0
            }
        };
        let mut window_y =
            (self.y as f64 + self.geometry.arrow_size + self.geometry.padding) as i32;
        if has_titlebar && self.has_titlebar {
            window_y += TITLE_BAR_HEIGHT;
        }
        (window_x, window_y)
    }

    fn get_level_icon(&self) -> &'static str {
        match self.level.as_str() {
            "error" => "🛑 ",
            "warning" => "⚠️ ",
            "info" => "ℹ️ ",
            _ => "",
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Geometry {
    padding: f64,
    radius: f64,
    arrow_size: f64,
    arrow_x: f64,
}

impl Geometry {
    fn from_env(env: &Env) -> Result<Self> {
        let padding_sym = env.intern("flycheck-gtk-tip-padding")?;
        let padding_value = env.call("symbol-value", [padding_sym])?;
        let padding = padding_value.into_rust::<u32>()? as f64;
        let radius_sym = env.intern("flycheck-gtk-tip-radius")?;
        let radius_value = env.call("symbol-value", [radius_sym])?;
        let radius = radius_value.into_rust::<u32>()? as f64;
        let arrow_size_sym = env.intern("flycheck-gtk-tip-arrow-size")?;
        let arrow_size_value = env.call("symbol-value", [arrow_size_sym])?;
        let arrow_size = arrow_size_value.into_rust::<u32>()? as f64;
        let arrow_x_sym = env.intern("flycheck-gtk-tip-arrow-x")?;
        let arrow_x_value = env.call("symbol-value", [arrow_x_sym])?;
        let arrow_x = arrow_x_value.into_rust::<u32>()? as f64;
        Ok(Self {
            padding,
            radius,
            arrow_size,
            arrow_x,
        })
    }
}
impl Default for Geometry {
    fn default() -> Self {
        Geometry {
            padding: 20.0,
            radius: 12.0,
            arrow_size: 14.0,
            arrow_x: 60.0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Shadow {
    padding: f64,
    steps: i32,
    dx: f64,
    dy: f64,
    color: gdk::RGBA,
}

impl Shadow {
    fn from_env(env: &Env) -> Result<Self> {
        let padding_sym = env.intern("flycheck-gtk-tip-shadow-padding")?;
        let padding_value = env.call("symbol-value", [padding_sym])?;
        let padding = padding_value.into_rust::<u32>()? as f64;

        let steps_sym = env.intern("flycheck-gtk-tip-shadow-steps")?;
        let steps_value = env.call("symbol-value", [steps_sym])?;
        let steps = steps_value.into_rust::<i32>()?;

        let dx_sym = env.intern("flycheck-gtk-tip-shadow-dx")?;
        let dx_value = env.call("symbol-value", [dx_sym])?;
        let dx = dx_value.into_rust::<u32>()? as f64;

        let dy_sym = env.intern("flycheck-gtk-tip-shadow-dy")?;
        let dy_value = env.call("symbol-value", [dy_sym])?;
        let dy = dy_value.into_rust::<u32>()? as f64;

        let color_sym = env.intern("flycheck-gtk-tip-shadow-color")?;
        let color_value = env.call("symbol-value", [color_sym])?;
        let color = color_value.into_rust::<String>()?;

        Ok(Self {
            padding,
            steps,
            dx,
            dy,
            color: gdk::RGBA::parse(&color)?,
        })
    }
}

impl Default for Shadow {
    fn default() -> Self {
        Shadow {
            padding: 24.0,
            steps: 10,
            dx: 5.0,
            dy: 10.0,
            color: gdk::RGBA::new(0.0, 0.0, 0.0, 1.0),
        }
    }
}

pub enum Event {
    HideTip,
    ShowTip(Tip),
}

pub struct TextCanvas {
    surface: ImageSurface,
    fg_color: String,
    bg_color: String,
    width: f64,
    height: f64,

    geometry: Geometry,
    shadow: Shadow,
}

impl Default for TextCanvas {
    fn default() -> Self {
        Self {
            surface: ImageSurface::create(Format::ARgb32, 1, 1).unwrap(),
            fg_color: "black".to_string(),
            bg_color: "white".to_string(),
            width: 1.0,
            height: 1.0,

            geometry: Geometry::default(),
            shadow: Shadow::default(),
        }
    }
}

impl TextCanvas {
    fn prepare_text(&mut self, tip: &Tip, max_width: i32) -> (f64, f64) {
        let txt = format!("{}{}", tip.get_level_icon(), tip.text);
        let tmp = ImageSurface::create(Format::ARgb32, 1, 1).unwrap();
        let cr = Context::new(&tmp).unwrap();
        let layout = pangocairo::functions::create_layout(&cr);
        layout.set_text(&txt);
        let desc = FontDescription::from_string(&format!("{} {}", &tip.font, &tip.font_size));
        layout.set_font_description(Some(&desc));
        layout.set_width(pango::SCALE * max_width);

        let (w, h) = layout.pixel_size();

        let surface = ImageSurface::create(Format::ARgb32, w, h).unwrap();
        let cr = Context::new(&surface).unwrap();
        let layout = pangocairo::functions::create_layout(&cr);

        layout.set_text(&txt);
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
        self.fg_color = tip.fg_color.clone();
        (self.width, self.height)
    }

    fn window_size(&self) -> (i32, i32) {
        (
            (self.full_width() + self.shadow.padding) as i32,
            (self.full_height() + self.shadow.padding + self.geometry.arrow_size) as i32,
        )
    }

    fn draw(&self, cr: &cairo::Context) {
        let width = self.full_width();
        let height = self.full_height();
        let geometry = self.geometry;

        cr.save();
        let arrow_half = geometry.arrow_size / 2.0;
        cr.new_path();
        cr.move_to(geometry.radius, geometry.arrow_size);
        cr.line_to(geometry.arrow_x - arrow_half, geometry.arrow_size);
        cr.line_to(geometry.arrow_x, 0.0);
        cr.line_to(geometry.arrow_x + arrow_half, geometry.arrow_size);
        cr.line_to(width - geometry.radius, geometry.arrow_size);

        cr.arc(
            width - geometry.radius,
            geometry.arrow_size + self.geometry.radius,
            geometry.radius,
            -std::f64::consts::FRAC_PI_2,
            0.0,
        );

        cr.line_to(width, height - self.geometry.radius);
        cr.arc(
            width - geometry.radius,
            height - geometry.radius,
            geometry.radius,
            0.0,
            std::f64::consts::FRAC_PI_2,
        );

        cr.line_to(geometry.radius, height);
        cr.arc(
            geometry.radius,
            height - geometry.radius,
            geometry.radius,
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::PI,
        );

        cr.line_to(0.0, geometry.arrow_size + geometry.radius);

        cr.arc(
            geometry.radius,
            geometry.arrow_size + geometry.radius,
            geometry.radius,
            std::f64::consts::PI,
            std::f64::consts::PI * 1.5,
        );
        cr.close_path();
        cr.restore();
    }

    fn full_width(&self) -> f64 {
        self.width + self.geometry.padding * 2.0
    }
    fn full_height(&self) -> f64 {
        self.height + self.geometry.padding * 2.0
    }
    fn draw_popover(&self, cr: &cairo::Context) {
        self.draw(cr);

        let bg_rgba = gdk::RGBA::parse(&self.bg_color).unwrap();

        cr.set_source_rgb(bg_rgba.red(), bg_rgba.green(), bg_rgba.blue());
        cr.fill_preserve();

        //final thin outline
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
        cr.set_line_width(1.0);
        cr.stroke();
    }
    fn draw_shadow(&self, cr: &cairo::Context) {
        for i in 0..self.shadow.steps {
            let t = i as f64 / self.shadow.steps as f64;

            let pad = t * self.geometry.padding;
            let alpha = (1.0 - t).powi(2) * 0.20;
            cr.save();
            cr.translate(self.shadow.dx - pad, self.shadow.dy - pad);
            cr.set_source_rgba(
                self.shadow.color.red(),
                self.shadow.color.green(),
                self.shadow.color.blue(),
                alpha,
            );

            self.draw(cr);
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
    let initialized = GTK_INITIALIZED.get_or_init(|| gtk::init().is_ok());

    if !*initialized {
        let _ = env.message("flychek-gtk-tip: failed to initialise gtk");
        return env.intern("t");
    }
    let _ = env.message("flychek-gtk-tip: gtk initialized");
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

    let screen = GtkWindowExt::screen(&window).unwrap();
    let rgba_visual = screen.rgba_visual().unwrap();
    window.set_visual(Some(&rgba_visual));
    window.set_app_paintable(true);

    window.set_transient_for(emacs_window.clone().as_ref());

    window.move_(0, 0);

    emacs_window.clone().map(|w| {
        let tip_window = window.clone();
        w.connect_focus_out_event(move |_win, _event| {
            tip_window.hide();
            gtk::glib::signal::Propagation::Proceed
        })
    });
    let area = gtk::DrawingArea::new();

    area.connect_draw({
        let canvas = canvas.clone();
        let window = window.clone();
        move |_, cr| {
            let canvas = canvas.borrow();

            let (window_w, window_h) = canvas.window_size();
            window.resize(window_w, window_h);
            canvas.draw_shadow(cr);
            canvas.draw_popover(cr);
            cr.set_source_surface(
                &*canvas.surface,
                canvas.geometry.padding,
                canvas.geometry.padding + canvas.geometry.arrow_size,
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

                    let mut canvas = canvas.borrow_mut();
                    canvas.geometry = tip.geometry;
                    canvas.shadow = tip.shadow;
                    canvas.prepare_text(&tip, max_width);

                    let (window_x, window_y) = tip.window_position(has_titlebar);
                    window.move_(window_x, window_y);

                    window.show_all();
                    area.queue_draw();

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
    level: String,
) -> Result<Value<'_>> {
    if let Some(lock) = SENDER.get() {
        let sender = lock.read().unwrap();
        let undecorated = env.intern("undecorated")?;
        let is_undecorated = env.call("frame-parameter", ((), undecorated))?;
        sender
            .send_blocking(Event::ShowTip(Tip {
                x,
                y,
                text,
                font,
                font_size,
                bg_color,
                fg_color,
                level,
                has_titlebar: is_undecorated == *nil,
                geometry: Geometry::from_env(env)?,
                shadow: Shadow::from_env(env)?,
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
