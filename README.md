# xdg-desktop-portal-phosh

A backend implementation for [xdg-desktop-portal][] that is using
GTK/GNOME/Phosh to provide interfaces that aren't provided by the GTK
portal.

There is also a Rust implementation under the binary name
`xdg-desktop-portal-phrosh` and D-BUS name
`org.freedesktop.impl.portal.desktop.phrosh`. Currently it provides
the following interfaces:

- `org.freedesktop.impl.portal.Account`
- `org.freedesktop.impl.portal.AppChooser`

## License

xdg-desktop-portal-phosh is licensed under the GPL-3.0-or-later license.

## Getting the source

```sh
git clone https://gitlab.gnome.org/guidog/xdg-desktop-portal-phosh.git
cd xdg-desktop-portal-phosh
```

The [main][] branch has the current development version.

## Dependencies

On a Debian based system run

```sh
sudo apt-get -y install build-essential
sudo apt-get -y build-dep .
```

For an explicit list of dependencies check the `Build-Depends` entry in the
[debian/control][] file.

## Building

We use the meson build system for xdg-desktop-portal-phosh. The quickest
way to get going is to do the following:

```sh
meson setup _build
meson compile -C _build
```

## Running

### Running from the source tree

After making sure `xdg-desktop-portal-phosh` isn't already running in your use session:

```sh
systemctl stop --user xdg-desktop-portal-phosh.service
systemctl stop --user xdg-desktop-portal-phrosh.service # For Phrosh
```

you can run it from the source tree:

```sh
G_MESSAGES_DEBUG=all _build/src/xdg-desktop-portal-phosh
G_MESSAGES_DEBUG=all _build/src/xdg-desktop-portal-phrosh # For Phrosh
```

## Getting in Touch

- Issue tracker: <https://gitlab.gnome.org/guidog/xdg-desktop-portal-phosh/issues>
- Matrix: <https://im.puri.sm/#/room/#phosh:talk.puri.sm>

[main]: https://gitlab.gnome.org/guidog/xdg-desktop-portal-phosh/-/tree/main
[.gitlab-ci.yml]: https://gitlab.gnome.org/guidog/xdg-desktop-portal-phosh/-/blob/main/.gitlab-ci.yml
[debian/control]: https://gitlab.gnome.org/guidog/xdg-desktop-portal-phosh/-/blob/main/debian/control
[xdg-desktop-portal]: http://github.com/flatpak/xdg-desktop-portal
