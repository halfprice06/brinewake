# dmgbuild settings for the macOS download, BRINEWAKE-VERSION-macos.dmg:
# BRINEWAKE and an Applications shortcut to drag it onto, over the background
# from dmg_background.py. tools/release.sh runs
#
#   dmgbuild -s tools/package/dmg.py -D app=PATH/BRINEWAKE.app \
#     -D background=PATH/background.tiff -D icon=tools/package/BRINEWAKE.icns \
#     BRINEWAKE OUT.dmg

import os

app = defines["app"]  # noqa: F821 (dmgbuild supplies defines)
name = os.path.basename(app)

format = "UDZO"
filesystem = "HFS+"
files = [app]
symlinks = {"Applications": "/Applications"}
hide_extension = [name]
icon = defines["icon"]  # noqa: F821
background = defines["background"]  # noqa: F821

# The background is 600x380 points; Finder counts the title bar in the
# window's height.
window_rect = ((200, 160), (600, 408))
default_view = "icon-view"
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
show_icon_preview = False
arrange_by = None
icon_size = 96
text_size = 13
# Twice the texel centres in dmg_background.py.
icon_locations = {name: (150, 184), "Applications": (450, 184)}
