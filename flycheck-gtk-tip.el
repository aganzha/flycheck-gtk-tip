;;; flycheck-gtk-tip.el --- Flycheck GTK tip  -*- lexical-binding: t; -*-

;;; Code:
(require 'flycheck)

;;; Installing:
;; (use-package flycheck-gtk-tip
;;   :straight (flycheck-gtk-tip
;;              :type git
;;              :local-repo "/home/aganzha/flycheck-gtk-tip/"))

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
          (flycheck-gtk-tip-show
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


(defun flycheck-gtk-tip-straight-setup ()
  (when (featurep 'pgtk)
    (let* ((module-name
            (file-name-base
             (directory-file-name
              (file-name-directory default-directory))))
           (soname (replace-regexp-in-string "-" "_" (format "lib%s.so" module-name)))
           (sopath
            (expand-file-name
             soname
             (expand-file-name module-name
                               (expand-file-name straight-build-dir
                                                 (expand-file-name "straight" user-emacs-directory))))))

      (unless (file-exists-p sopath)
        (url-copy-file (format "http://localhost:9000/%s" soname) sopath t))
      (module-load sopath)

      (setq flycheck-display-errors-function #'flycheck-gtk-tip-display-errors-function)
      (setq flycheck-clear-displayed-errors-function #'flycheck-gtk-tip-hide)
      (setq flycheck-display-errors-delay 0.2)
      (advice-add 'keyboard-quit :before
                  (defun kill-gtk-tip (&rest _)
                    (flycheck-gtk-tip-hide))))))

(flycheck-gtk-tip-straight-setup)

(provide 'flycheck-gtk-tip)
;;; flycheck-gtk-tip.el ends here
