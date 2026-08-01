## Installing
```elisp
(use-package flycheck-gtk-tip
  :vc (:url "https://github.com/aganzha/flycheck-gtk-tip"))
```
<img width="1612" height="1080" alt="image" src="https://github.com/user-attachments/assets/31a55970-0067-4cc5-87ee-62926c282b0f" />
<img width="1612" height="1080" alt="image" src="https://github.com/user-attachments/assets/e819f63d-2f3b-492b-a53a-efd8fa14e004" />

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
