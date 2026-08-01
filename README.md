+## Installing
```elisp
(use-package flycheck-gtk-tip
  :vc (:url "https://github.com/aganzha/flycheck-gtk-tip"))
```
<img width="1920" height="1052" alt="dark theme" src="https://github.com/user-attachments/assets/31274bf6-120e-45f1-a740-fb9ca41bbbd5" />

<img width="1920" height="1052" alt="light-theme" src="https://github.com/user-attachments/assets/395b2640-686d-4597-b51a-e0f2dc95d93c" />

## Using
Pop Up tip appears when cursor is on error string. `C-g` to force close tip.

## Customizing
There are a couple of variables in ```flycheck-gtk-tip``` group, which could be customized.
One of them: ```flycheck-gtk-tip-vertical-offset``` could be used to adjust popup tip vertically, cause this gap depends on emacs window decorations used.

## Uninstalling/Updating
Just delete folder ```~/.emacs.d/elpa/flycheck-gtk-tip```.
Having 
```elisp
(use-package flycheck-gtk-tip
  :vc (:url "https://github.com/aganzha/flycheck-gtk-tip"))
```
in your .emacs/init.el will bring latest version from github.
