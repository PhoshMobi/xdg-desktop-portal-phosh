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

generated_sources += gnome.compile_resources('pmp-resources', 'pmp.gresources.xml', c_name: 'pmp')

pmp_sources = files(
  'fc-monitor.c',
  'fc-monitor.h',
  'pmp-external-win.c',
  'pmp-external-win.h',
  'pmp-fdo-notification.c',
  'pmp-fdo-notification.h',
  'pmp-file-chooser.c',
  'pmp-file-chooser.h',
  'pmp-notification.c',
  'pmp-notification.h',
  'pmp-request.c',
  'pmp-request.h',
  'pmp-settings.c',
  'pmp-settings.h',
  'pmp-utils.c',
  'pmp-utils.h',
  'pmp-wallpaper-dialog.c',
  'pmp-wallpaper-dialog.h',
  'pmp-wallpaper-preview.c',
  'pmp-wallpaper-preview.h',
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
  libpfs_dep,
  xdg_desktop_portal_dep,
]

pmp_dep = declare_dependency(
  sources: generated_sources,
  include_directories: root_inc,
  dependencies: pmp_deps,
)

pmp = executable(
  'xdg-desktop-portal-phosh',
  pmp_sources,
  dependencies: pmp_dep,
  install: true,
  install_dir: libexecdir,
)

resources = gnome.compile_resources(
  '@0@'.format(phrosh_exe_name),
  '@0@.gresource.xml'.format(phrosh_exe_name),
  gresource_bundle: true,
)

conf = configuration_data()
conf.set_quoted('GETTEXT_PACKAGE', 'phosh-mobile-portal')
conf.set_quoted('LOCALE_DIR', get_option('prefix') / get_option('localedir'))
conf.set_quoted('DBUS_NAME', phrosh_dbus_name)

configure_file(input: 'lib_config.rs.in', output: 'lib_config.rs', configuration: conf)

run_command(
  'cp',
  meson.project_build_root() / 'src' / 'lib_config.rs',
  meson.project_source_root() / 'src' / 'lib_config.rs',
  check: true,
)

configure_file(input: 'bin_config.rs.in', output: 'bin_config.rs', configuration: conf)

run_command(
  'cp',
  meson.project_build_root() / 'src' / 'bin_config.rs',
  meson.project_source_root() / 'src' / 'bin_config.rs',
  check: true,
)

cargo_bin = find_program('cargo')

cargo_opt = ['--manifest-path', meson.project_source_root() / 'Cargo.toml']
cargo_opt += ['--target-dir', meson.project_build_root() / 'src']

if get_option('buildtype') == 'release'
  cargo_opt += ['--release']
  cargo_target = 'release'
else
  cargo_target = 'debug'
endif

cargo_env = environment()
cargo_env.set('CARGO_HOME', meson.project_build_root() / 'cargo')
cargo_env.set('RESOURCES_DIR', meson.current_build_dir())

if get_option('rust-portal')
  custom_target(
    phrosh_exe_name,
    build_by_default: true,
    build_always_stale: true,
    console: true,
    install: true,
    install_dir: libexecdir,
    env: cargo_env,
    output: phrosh_exe_name,
    command: [
      cargo_bin,
      'build',
      cargo_opt,
      '&&',
      'cp',
      meson.project_build_root() / 'src' / cargo_target / pmp_exe_name,
      '@OUTPUT@',
    ],
  )

  cargo_opt = ['test'] + cargo_opt
  test(phrosh_exe_name, cargo_bin, args: cargo_opt, env: cargo_env)
endif

subdir('thumbnailer')
