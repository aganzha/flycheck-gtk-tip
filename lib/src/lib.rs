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

#[defun]
fn show_window(env: &Env) -> Result<Value<'_>> {
    INIT.call_once(|| {
        let _ = gtk::init();
    });
    let (sender, receiver) = async_channel::unbounded();
    SENDER.get_or_init(|| RwLock::new(sender.clone()));
    let win = gtk::Window::new(gtk::WindowType::Toplevel);
    win.set_title("From Emacs (GTK3)");
    win.set_default_size(400, 300);

    // let btn = gtk::Button::with_label("Click me");
    // btn.connect_clicked({
    //     //let window = win.clone();
    //     let sender = sender.clone();
    //     move |_| {
    //         eprintln!("Button clicked from Emacs module");
    //         //eprintln!("window! {:?}", window);
    //         sender
    //             .send_blocking(Event::Test)
    //             .expect("cant send through channel");
    //     }
    // });
    //win.add(&btn);
    let surface = render_text_offscreen("Hello from thread", "Sans", 24.0);
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
    win.add(&area);
    win.show_all();

    glib::spawn_future_local(async move {
        while let Ok(event) = receiver.recv().await {
            match event {
                Event::Test => {
                    eprintln!("🐦 xtest vent! {:?}", win);
                }
                Event::Best(title) => {
                    win.set_title(&title);
                    eprintln!("🧣 BEST vent! {:?} and title {:?}", win, title);
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
