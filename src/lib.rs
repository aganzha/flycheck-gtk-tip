// attemp1
// it works!
// ;;(module-load (expand-file-name "/home/aganzha/emacs-gtk3-module/target/release/libemacs_gtk3_module.so"))
// ;;(emacs-gtk3-module-show-window)

use emacs::{defun, Env, Result, Value};
use gtk::prelude::*;
use std::sync::Once;

emacs::plugin_is_GPL_compatible!();

static INIT: Once = Once::new();

#[emacs::module(name = "emacs-gtk3-module")]
fn init(_env: &Env) -> Result<()> {
    Ok(())
}

#[defun]
fn show_window(env: &Env) -> Result<Value<'_>> {
    INIT.call_once(|| {
        // Try to init, ignore failure if already initialized
        let _ = gtk::init();
        eprintln!("just inited gtk3. how?");
    });

    let win = gtk::Window::new(gtk::WindowType::Toplevel);
    win.set_title("From Emacs (GTK3)");
    win.set_default_size(400, 300);

    let btn = gtk::Button::with_label("Click me");
    btn.connect_clicked(|_| {
        eprintln!("Button clicked from Emacs module");
    });
    win.add(&btn);

    win.show_all();

    Ok(env.intern("t")?)
}

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
