;;; flycheck-gtk-tip.el --- Flycheck GTK tip  -*- lexical-binding: t; -*-

;;; Code:
(require 'flycheck)

;;; Installing:
;; (use-package flycheck-gtk-tip
;;   :straight (flycheck-gtk-tip
;;              :type git
;;              :local-repo "/home/aganzha/emacs-gtk3-module/"
;;              :pre-build ("curl" "-L" "-O" "https://github.com/agrahn/Android-Password-Store/releases/download/latest/rev-hash.txt")
;;              :files ("rev-hash.txt" "flycheck-gtk-tip.el"))
;;   :ensure t
;;   :config (flycheck-gtk-tip-setup))

(defun flycheck-gtk-tip-display-errors-function (errors)
  (let ((all-messages ""))
    (dolist (err errors)
      (let ((my-buffer-name (buffer-file-name))
            (err-buffer-name (buffer-file-name (flycheck-error-buffer err)))
            (my-line (line-number-at-pos))
            (err-line (flycheck-error-line err))
            (message (flycheck-error-message err))
            )
        (if (and (eq my-buffer-name err-buffer-name)
                 (eq my-line err-line))
            (setq all-messages (concat all-messages message "\n"))
          )))
    (if (not (string-empty-p all-messages))
        (let ((pos (window-absolute-pixel-position))
              (font-family (symbol-name (font-get (face-attribute 'default :font) :family)))
              (font-size (aref (font-info (face-font 'default)) 2))
              (font-scale (cl-reduce 'max (mapcar #'cdr face-font-rescale-alist) :initial-value 1))
              (fg-color (face-attribute 'default :foreground))
              (bg-color (face-attribute 'default :background)))
          (emacs-gtk3-module-show-tip
           (car pos)
           (cdr pos)
           all-messages
           font-family
           (/ font-size font-scale)
           fg-color
           bg-color
           )
          )
      )
    )
  )

(defun flycheck-gtk-tip-setup ()
  (let* ((dir-name (expand-file-name "libemacs_gtk3_module" user-emacs-directory))
         (soname (expand-file-name "libemacs_gtk3_module.so" dir-name)))
    ;;(module-load soname)
    (module-load "/home/aganzha/emacs-gtk3-module/target/release/libemacs_gtk3_module.so")
    (setq flycheck-display-errors-function #'flycheck-gtk-tip-display-errors-function)  
    (setq flycheck-clear-displayed-errors-function #'emacs-gtk3-module-hide-tip)
    (setq flycheck-display-errors-delay 0.2)
    (advice-add 'keyboard-quit :before
                (defun kill-gtk-tip (&rest _)
                  (emacs-gtk3-module-hide-tip)))
    )
  )

(provide 'flycheck-gtk-tip)
;;; flycheck-gtk-tip.el ends here
