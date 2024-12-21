project('xdg-desktop-portal-phosh', 'c',
          version: '0.44~rc2',
          license: 'GPL-3.0-or-later',
    meson_version: '>= 1.0.0',
  default_options: [ 'warning_level=1', 'buildtype=debugoptimized', 'c_std=gnu11' ],
)

pmp_exe_name = 'xdg-desktop-portal-phosh'
pmp_dbus_name = 'org.freedesktop.impl.portal.desktop.phosh'
pmp_portals = [
  'org.freedesktop.impl.portal.FileChooser',
  'org.freedesktop.impl.portal.Settings',
  'org.freedesktop.impl.portal.Wallpaper',
]

prefix = get_option('prefix')
datadir = prefix / get_option('datadir')
libdir = get_option('libdir')
libexecdir = prefix / get_option('libexecdir')
localedir = prefix / get_option('localedir')
desktopdir = datadir / 'applications'
servicedir = datadir / 'dbus-1' / 'services'
portaldir = datadir / 'xdg-desktop-portal' / 'portals'

glib_ver = '2.74'
glib_ver_str = 'GLIB_VERSION_@0@'.format(glib_ver.replace('.','_'))
glib_ver_cmp = '>=@0@'.format(glib_ver)

gtk_ver = '4.12'
gtk_ver_str = 'GDK_VERSION_@0@'.format(gtk_ver.replace('.','_'))
gtk_ver_cmp = '>=@0@'.format(gtk_ver)

adw_ver = '1.6'
adw_ver_str = 'ADW_VERSION_@0@'.format(adw_ver.replace('.','_'))
adw_ver_cmp = '>=@0@'.format(adw_ver)

root_inc = include_directories('.')

cc = meson.get_compiler('c')
global_c_args = []
test_c_args = [
  '-Wcast-align',
  '-Wdate-time',
  '-Wdeclaration-after-statement',
  ['-Werror=format-security', '-Werror=format=2'],
  '-Wendif-labels',
  '-Werror=incompatible-pointer-types',
  '-Werror=missing-declarations',
  '-Werror=overflow',
  '-Werror=return-type',
  '-Werror=shift-count-overflow',
  '-Werror=shift-overflow=2',
  '-Werror=implicit-fallthrough=3',
  '-Wfloat-equal',
  '-Wformat-nonliteral',
  '-Wformat-security',
  '-Winit-self',
  '-Wmaybe-uninitialized',
  '-Wmissing-field-initializers',
  '-Wmissing-include-dirs',
  '-Wmissing-noreturn',
  '-Wnested-externs',
  '-Wno-missing-field-initializers',
  '-Wno-sign-compare',
  '-Wno-strict-aliasing',
  '-Wno-unused-parameter',
  '-Wold-style-definition',
  '-Wpointer-arith',
  '-Wredundant-decls',
  '-Wshadow',
  '-Wstrict-prototypes',
  '-Wswitch-default',
  '-Wswitch-enum',
  '-Wtype-limits',
  '-Wundef',
  '-Wunused-function',
]
if get_option('buildtype') != 'plain'
  test_c_args += '-fstack-protector-strong'
endif

foreach arg: test_c_args
  if cc.has_multi_arguments(arg)
    global_c_args += arg
  endif
endforeach
add_project_arguments(
  [
    global_c_args,
    '-DGLIB_VERSION_MIN_REQUIRED=@0@'.format(glib_ver_str),
    '-DGLIB_VERSION_MAX_ALLOWED=@0@'.format(glib_ver_str),
    '-DGDK_VERSION_MIN_REQUIRED=@0@'.format(gtk_ver_str),
    '-DGDK_VERSION_MAX_ALLOWED=@0@'.format(gtk_ver_str),
    '-DADW_VERSION_MIN_REQUIRED=@0@'.format(adw_ver_str),
    '-DADW_VERSION_MAX_ALLOWED=@0@'.format(adw_ver_str),
  ],
  language: 'c',
)

gnome = import('gnome')
i18n  = import('i18n')
pkgconfig = import ('pkgconfig')

adwaita_dep = dependency('libadwaita-1', version: adw_ver_cmp)
fontconfig_dep = dependency('fontconfig')
gio_dep = dependency('gio-2.0', version: glib_ver_cmp)
gio_unix_dep =  dependency('gio-unix-2.0', version: glib_ver_cmp)
glib_dep = dependency('glib-2.0', version: glib_ver_cmp)
gnome_bg_dep = dependency('gnome-bg-4', version: '>=43')
gnome_desktop_dep = dependency('gnome-desktop-4', version: '>=43')
gsettings_desktop_schemas_dep = dependency('gsettings-desktop-schemas', version: '>= 47')
gtk_dep = dependency('gtk4', version: gtk_ver_cmp)
gtk_wayland_dep = dependency('gtk4-wayland', version: gtk_ver_cmp)
libpfs_dep = dependency('libpfs-0', version: '>= 0',
                        fallback: ['pfs', 'pfs_dep'],
                        default_options:
                          [ 'examples=false',
                            'shared-lib=false',
                          ])
xdg_desktop_portal_dep = dependency('xdg-desktop-portal', version: '>= 1.14.0')

systemd_user_unit_dir = get_option('systemd_user_unit_dir')
systemd_dep = dependency('systemd', version: '>= 242', required: false)
if systemd_user_unit_dir == ''
  if systemd_dep.found()
    systemd_user_unit_dir = systemd_dep.get_variable(pkgconfig: 'systemduserunitdir')
  else
    systemd_user_unit_dir = prefix / libdir / 'systemd' / 'user'
  endif
endif

config_h = configuration_data()
config_h.set_quoted('GETTEXT_PACKAGE', 'phosh-mobile-portal')
config_h.set_quoted('LOCALEDIR', localedir)
config_h.set_quoted('PACKAGE_STRING', '@0@ @1@'.format(meson.project_name(), meson.project_version()))
config_h.set_quoted('PMP_DBUS_NAME', pmp_dbus_name)

configure_file(
  output: 'pmp-config.h',
  configuration: config_h,
)


subdir('data')
subdir('po')
subdir('src')
