xdp_interfaces_dir = xdg_desktop_portal_dep.get_variable(pkgconfig: 'interfaces_dir')

# XDG Desktop Portal D-Bus interfaces
xdp_interface_files = [
  xdp_interfaces_dir / 'org.freedesktop.impl.portal.Request.xml',
  xdp_interfaces_dir / 'org.freedesktop.impl.portal.Session.xml',
]

foreach portal : pmp_portals
  xdp_interface_files += xdp_interfaces_dir / '@0@.xml'.format(portal)
endforeach

generated_sources = gnome.gdbus_codegen(
  'xdg-desktop-portal-dbus',
  sources: xdp_interface_files,
  interface_prefix: 'org.freedesktop.impl.portal.',
  namespace: 'PmpImpl',
)

generated_sources += gnome.compile_resources(
   'pmp-resources',
   'pmp.gresources.xml',
   c_name: 'pmp',
)

pmp_sources = files(
  'fc-monitor.c',
  'fc-monitor.h',
  'pmp-external-win.c',
  'pmp-external-win.h',
  'pmp-request.c',
  'pmp-request.h',
  'pmp-settings.c',
  'pmp-settings.h',
  'pmp-utils.c',
  'pmp-utils.h',
  'pmp-wallpaper-preview.c',
  'pmp-wallpaper-preview.h',
  'pmp-wallpaper-dialog.c',
  'pmp-wallpaper-dialog.h',
  'pmp-wallpaper.c',
  'pmp-wallpaper.h',
  'xdg-desktop-portal-phosh.c',
)

pmp_deps = [
  adwaita_dep,
  fontconfig_dep,
  glib_dep,
  gio_dep,
  gio_unix_dep,
  gsettings_desktop_schemas_dep,
  gtk_dep,
  gtk_wayland_dep,
  gnome_bg_dep,
  gnome_desktop_dep,
  xdg_desktop_portal_dep,
]

pmp_dep = declare_dependency(
  sources: generated_sources,
  include_directories: root_inc,
  dependencies: pmp_deps)

pmp = executable('xdg-desktop-portal-phosh',
  pmp_sources,
  dependencies: pmp_dep,
  install: true,
  install_dir: libexecdir)
